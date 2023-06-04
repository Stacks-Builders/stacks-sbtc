use blockstack_lib::{
    address::{C32_ADDRESS_VERSION_MAINNET_SINGLESIG, C32_ADDRESS_VERSION_TESTNET_SINGLESIG},
    chainstate::stacks::TransactionVersion,
};

pub fn address_version(version: &TransactionVersion) -> u8 {
    match version {
        TransactionVersion::Mainnet => C32_ADDRESS_VERSION_MAINNET_SINGLESIG,
        TransactionVersion::Testnet => C32_ADDRESS_VERSION_TESTNET_SINGLESIG,
    }
}

#[cfg(test)]
pub mod test {
    use blockstack_lib::{
        burnchains::{
            bitcoin::{
                address::{BitcoinAddress, SegwitBitcoinAddress},
                BitcoinTransaction, BitcoinTxInput, BitcoinTxOutput,
            },
            BurnchainBlockHeader, BurnchainTransaction, PrivateKey, Txid,
        },
        chainstate::burn::{operations::PegOutRequestOp, Opcodes},
        util::{hash::Sha256Sum, secp256k1::Secp256k1PrivateKey},
    };
    use rand::Rng;

    pub const PRIVATE_KEY_HEX: &str =
        "b244296d5907de9864c0b0d51f98a13c52890be0404e83f273144cd5b9960eed01";
    pub const PUBLIC_KEY_HEX: &str =
        "cc8a4bc64d897bddc5fbc2f670f7a8ba0b386779106cf1223c6fc5d7cd6fc115";

    /// Helper function to construct a valid signed peg out request op
    pub fn build_peg_out_request_op(
        key_hex: &str,
        amount: u64,
        dust_amount: u64,
        fulfillment_fee: u64,
    ) -> PegOutRequestOp {
        let mut rng = rand::thread_rng();
        let private_key = Secp256k1PrivateKey::from_hex(key_hex)
            .expect("Failed to construct a valid private key.");

        // Build a dust txo
        let recipient_address_bytes = rng.gen::<[u8; 32]>();
        let output2 = BitcoinTxOutput {
            units: dust_amount,
            address: BitcoinAddress::Segwit(SegwitBitcoinAddress::P2TR(
                true,
                recipient_address_bytes,
            )),
        };

        // Build a fulfillment fee txo
        let peg_wallet_address = rng.gen::<[u8; 32]>();
        let output3 = BitcoinTxOutput {
            units: fulfillment_fee,
            address: BitcoinAddress::Segwit(SegwitBitcoinAddress::P2TR(true, peg_wallet_address)),
        };

        // Generate the message signature by signing the amount and recipient fields
        let mut script_pubkey = vec![81, 32]; // OP_1 OP_PUSHBYTES_32
        script_pubkey.extend_from_slice(&recipient_address_bytes);

        let mut msg = amount.to_be_bytes().to_vec();
        msg.extend_from_slice(&script_pubkey);

        let signature = private_key
            .sign(Sha256Sum::from_data(&msg).as_bytes())
            .expect("Failed to sign amount and recipient fields.");

        let mut data = vec![];
        data.extend_from_slice(&amount.to_be_bytes());
        data.extend_from_slice(signature.as_bytes());

        let outputs = vec![output2, output3];
        let inputs = vec![];

        // Build the burnchain tx using the above generated data
        let tx = build_burnchain_transaction(Opcodes::PegOutRequest as u8, data, inputs, outputs);

        // Build an empty block header
        let header = build_empty_block_header();

        // use the header and tx to generate a peg out request
        PegOutRequestOp::from_tx(&header, &tx).expect("Failed to construct peg-out request op")
    }

    fn build_empty_block_header() -> BurnchainBlockHeader {
        BurnchainBlockHeader {
            block_height: 0,
            block_hash: [0; 32].into(),
            parent_block_hash: [0; 32].into(),
            num_txs: 0,
            timestamp: 0,
        }
    }

    fn build_burnchain_transaction(
        opcode: u8,
        data: Vec<u8>,
        inputs: Vec<BitcoinTxInput>,
        outputs: Vec<BitcoinTxOutput>,
    ) -> BurnchainTransaction {
        BurnchainTransaction::Bitcoin(BitcoinTransaction {
            txid: Txid([0; 32]),
            vtxindex: 0,
            opcode,
            data,
            data_amt: 0,
            inputs,
            outputs,
        })
    }
}
