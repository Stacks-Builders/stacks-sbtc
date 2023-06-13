use crate::{
    peg_wallet::{Error as PegWalletError, StacksWallet as StacksWalletTrait},
    stacks_node::{PegInOp, PegOutRequestOp},
    util::address_version,
};
use bitcoin::secp256k1::PublicKey;
use blockstack_lib::{
    chainstate::stacks::{
        StacksTransaction, StacksTransactionSigner, TransactionAnchorMode, TransactionAuth,
        TransactionContractCall, TransactionPayload, TransactionPostConditionMode,
        TransactionSpendingCondition, TransactionVersion,
    },
    core::{CHAIN_ID_MAINNET, CHAIN_ID_TESTNET},
    types::chainstate::{StacksAddress, StacksPrivateKey, StacksPublicKey},
    vm::{
        errors::RuntimeErrorType,
        types::{ASCIIData, BuffData, SequenceData, StacksAddressExtensions, TupleData},
        ClarityName, ContractName, Value,
    },
};

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum Error {
    ///Error due to invalid configuration values
    #[error("{0}")]
    ConfigError(String),
    ///Error occured while signing a transaction
    #[error("Failed to sign transaction: {0}")]
    SigningError(String),
    ///Error occurred due to a malformed op
    #[error("{0}")]
    MalformedOp(String),
    ///Error occurred at Clarity runtime
    #[error("Clarity runtime error ocurred: {0}")]
    ClarityRuntimeError(#[from] RuntimeErrorType),
    ///Blockstack error
    #[error("Clarity runtime error ocurred: {0}")]
    BlockstackError(#[from] blockstack_lib::vm::errors::Error),
}

pub struct StacksWallet {
    contract_address: StacksAddress,
    contract_name: ContractName,
    sender_key: StacksPrivateKey,
    public_key: StacksPublicKey,
    address: StacksAddress,
    version: TransactionVersion,
    fee: u64,
}

impl StacksWallet {
    pub fn new(
        contract_name: ContractName,
        contract_address: StacksAddress,
        sender_key: StacksPrivateKey,
        address: StacksAddress,
        version: TransactionVersion,
        fee: u64,
    ) -> Self {
        let public_key = StacksPublicKey::from_private(&sender_key);
        Self {
            contract_address,
            contract_name,
            sender_key,
            public_key,
            address,
            version,
            fee,
        }
    }

    fn build_transaction_signed(
        &self,
        function_name: impl Into<String>,
        function_args: Vec<Value>,
        nonce: u64,
    ) -> Result<StacksTransaction, Error> {
        // First build an unsigned transaction
        let unsigned_tx = self.build_transaction_unsigned(function_name, function_args, nonce)?;

        // Do the signing
        let mut tx_signer = StacksTransactionSigner::new(&unsigned_tx);
        tx_signer
            .sign_origin(&self.sender_key)
            .map_err(|e| Error::SigningError(e.to_string()))?;

        // Retrieve the signed transaction from the signer
        let signed_tx = tx_signer.get_tx().ok_or(Error::SigningError(
            "Unable to retrieve signed transaction from the signer.".to_string(),
        ))?;
        Ok(signed_tx)
    }

    fn build_transaction_unsigned(
        &self,
        function_name: impl Into<String>,
        function_args: Vec<Value>,
        nonce: u64,
    ) -> Result<StacksTransaction, Error> {
        // First build the payload from the provided function and its arguments
        let payload = self.build_transaction_payload(function_name, function_args)?;

        // Next build the authorization from the provided sender key
        let public_key = self.public_key();
        let mut spending_condition = TransactionSpendingCondition::new_singlesig_p2pkh(*public_key)
            .ok_or_else(|| {
                Error::SigningError(
                    "Failed to create transaction spending condition from provided sender_key."
                        .to_string(),
                )
            })?;
        spending_condition.set_nonce(nonce);
        spending_condition.set_tx_fee(self.fee);
        let auth = TransactionAuth::Standard(spending_condition);

        // Viola! We have an unsigned transaction
        let mut unsigned_tx = StacksTransaction::new(self.version, auth, payload);
        unsigned_tx.anchor_mode = TransactionAnchorMode::Any;
        unsigned_tx.post_condition_mode = TransactionPostConditionMode::Allow;
        unsigned_tx.chain_id = if self.version == TransactionVersion::Testnet {
            CHAIN_ID_TESTNET
        } else {
            CHAIN_ID_MAINNET
        };

        Ok(unsigned_tx)
    }

    fn build_transaction_payload(
        &self,
        function_name: impl Into<String>,
        function_args: Vec<Value>,
    ) -> Result<TransactionPayload, RuntimeErrorType> {
        let function_name = ClarityName::try_from(function_name.into())?;
        let payload = TransactionContractCall {
            address: self.contract_address,
            contract_name: self.contract_name.clone(),
            function_name,
            function_args,
        };
        Ok(payload.into())
    }
}

impl StacksWalletTrait for StacksWallet {
    fn build_mint_transaction(
        &self,
        op: &PegInOp,
        nonce: u64,
    ) -> Result<StacksTransaction, PegWalletError> {
        let function_name = "mint!";

        // Build the function arguments
        let amount = Value::UInt(op.amount.into());
        let principal = Value::from(op.recipient.clone());
        //Note that this tx_id is only used to print info in the contract call.
        let tx_id = Value::from(ASCIIData {
            data: op.txid.to_string().as_bytes().to_vec(),
        });
        let function_args: Vec<Value> = vec![amount, principal, tx_id];
        let tx = self.build_transaction_signed(function_name, function_args, nonce)?;
        Ok(tx)
    }

    fn build_burn_transaction(
        &self,
        op: &PegOutRequestOp,
        nonce: u64,
    ) -> Result<StacksTransaction, PegWalletError> {
        let function_name = "burn!";

        // Build the function arguments
        let amount = Value::UInt(op.amount.into());
        // Retrieve the stacks address to burn from
        let address = op
            .stx_address(address_version(&self.version))
            .map_err(|_| {
                Error::MalformedOp(
                    "Failed to recover stx address from peg-out request op.".to_string(),
                )
            })?;
        let principal_data = address.to_account_principal();
        let principal = Value::Principal(principal_data);
        //Note that this tx_id is only used to print info inside the contract call.
        let tx_id = Value::from(ASCIIData {
            data: op.txid.to_string().as_bytes().to_vec(),
        });
        let function_args: Vec<Value> = vec![amount, principal, tx_id];

        let tx = self.build_transaction_signed(function_name, function_args, nonce)?;
        Ok(tx)
    }

    fn build_set_bitcoin_wallet_public_key_transaction(
        &self,
        public_key: &PublicKey,
        nonce: u64,
    ) -> Result<StacksTransaction, PegWalletError> {
        let function_name = "set-bitcoin-wallet-public-key";
        // Build the function arguments
        let key = Value::Sequence(SequenceData::Buffer(BuffData {
            data: public_key.serialize().to_vec(),
        }));
        let function_args = vec![key];
        let tx = self.build_transaction_signed(function_name, function_args, nonce)?;
        Ok(tx)
    }

    fn build_set_coordinator_data_transaction(
        &self,
        address: &StacksAddress,
        public_key: &StacksPublicKey,
        nonce: u64,
    ) -> Result<StacksTransaction, PegWalletError> {
        let function_name = "set-coordinator-data";
        let principal = Value::Principal(address.to_account_principal());
        let key = Value::Sequence(SequenceData::Buffer(BuffData {
            data: public_key.to_bytes_compressed(),
        }));
        let data = TupleData::from_data(vec![
            (ClarityName::from("addr"), principal),
            (ClarityName::from("key"), key),
        ])
        .map_err(Error::from)?;
        let function_args = vec![data.into()];
        let tx = self.build_transaction_signed(function_name, function_args, nonce)?;
        Ok(tx)
    }

    fn address(&self) -> &StacksAddress {
        &self.address
    }

    fn public_key(&self) -> &StacksPublicKey {
        &self.public_key
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        peg_wallet::StacksWallet as StacksWalletTrait,
        stacks_wallet::StacksWallet,
        util::{
            address_version,
            test::{build_peg_out_request_op, PRIVATE_KEY_HEX, PUBLIC_KEY_HEX},
        },
    };
    use bitcoin::{secp256k1::Parity, XOnlyPublicKey};
    use blockstack_lib::{
        address::{AddressHashMode, C32_ADDRESS_VERSION_TESTNET_SINGLESIG},
        burnchains::{Address, Txid},
        chainstate::{
            burn::operations::{PegInOp, PegOutRequestOp},
            stacks::{address::PoxAddress, TransactionVersion},
        },
        types::chainstate::{
            BurnchainHeaderHash, StacksAddress, StacksPrivateKey, StacksPublicKey,
        },
        util::{hash::Hash160, secp256k1::MessageSignature},
        vm::{
            types::{PrincipalData, StandardPrincipalData},
            ContractName,
        },
    };
    use rand::Rng;
    use std::str::FromStr;

    fn pox_address() -> PoxAddress {
        PoxAddress::Standard(
            StacksAddress::new(
                C32_ADDRESS_VERSION_TESTNET_SINGLESIG,
                Hash160::from_data(&rand::thread_rng().gen::<[u8; 20]>()),
            ),
            None,
        )
    }

    fn stacks_wallet() -> StacksWallet {
        let sender_key =
            StacksPrivateKey::from_hex(PRIVATE_KEY_HEX).expect("Failed to parse private key");

        let pk = StacksPublicKey::from_private(&sender_key);

        let address = StacksAddress::from_public_keys(
            address_version(&TransactionVersion::Testnet),
            &AddressHashMode::SerializeP2PKH,
            1,
            &vec![pk],
        )
        .expect("Failed to create stacks address");
        let contract_name = ContractName::from("sbtc-alpha");

        let contract_address =
            StacksAddress::from_string("SP3FBR2AGK5H9QBDH3EEN6DF8EK8JY7RX8QJ5SVTE")
                .expect("Failed to parse contract address");
        StacksWallet::new(
            contract_name,
            contract_address,
            sender_key,
            address,
            TransactionVersion::Testnet,
            10,
        )
    }

    #[test]
    fn build_mint_transaction_test() {
        let p = PegInOp {
            recipient: PrincipalData::Standard(StandardPrincipalData(0, [0u8; 20])),
            peg_wallet_address: pox_address(),
            amount: 55155,
            memo: Vec::default(),
            txid: Txid([0u8; 32]),
            vtxindex: 0,
            block_height: 0,
            burn_header_hash: BurnchainHeaderHash([0; 32]),
        };
        let wallet = stacks_wallet();
        let tx = wallet
            .build_mint_transaction(&p, 0)
            .expect("Failed to construct mint transaction.");
        tx.verify()
            .expect("build_mint_transaction generated a transaction with an invalid signature");
    }

    #[test]
    fn build_burn_transaction_test() {
        let wallet = stacks_wallet();
        let op = build_peg_out_request_op(PRIVATE_KEY_HEX, 10, 1, 3);
        let tx = wallet
            .build_burn_transaction(&op, 0)
            .expect("Failed to construct burn transaction.");
        tx.verify()
            .expect("build_burn_transaction generated a transaction with an invalid signature.");
    }

    #[test]
    fn invalid_burn_op_test() {
        let wallet = stacks_wallet();
        // Construct an invalid peg-out request op.
        let op = PegOutRequestOp {
            amount: 1000,
            recipient: pox_address(),
            signature: MessageSignature([0x00; 65]),
            peg_wallet_address: pox_address(),
            fulfillment_fee: 0,
            memo: vec![],
            txid: Txid([0x04; 32]),
            vtxindex: 0,
            block_height: 0,
            burn_header_hash: BurnchainHeaderHash([0x00; 32]),
        };
        assert_eq!(
            wallet
                .build_burn_transaction(&op, 0)
                .err()
                .unwrap()
                .to_string(),
            "Stacks Wallet Error: Failed to recover stx address from peg-out request op."
        );
    }

    #[test]
    fn build_set_bitcoin_wallet_public_key_transaction_test() {
        let wallet = stacks_wallet();
        let internal_key = XOnlyPublicKey::from_str(PUBLIC_KEY_HEX).unwrap();
        let public_key = internal_key.public_key(Parity::Even);

        let tx = wallet
            .build_set_bitcoin_wallet_public_key_transaction(&public_key, 0)
            .expect("Failed to construct a set btc address transaction.");
        tx.verify().expect(
            "build_set_btc_address_transaction generated a transaction with an invalid signature.",
        );
    }
}
