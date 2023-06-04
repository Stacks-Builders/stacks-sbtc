use std::time::{Duration, Instant};

use crate::stacks_node::{Error as StacksNodeError, PegInOp, PegOutRequestOp, StacksNode};
use bitcoin::XOnlyPublicKey;
use blockstack_lib::{
    chainstate::stacks::StacksTransaction,
    codec::StacksMessageCodec,
    types::chainstate::StacksAddress,
    vm::{types::SequenceData, ClarityName, ContractName, Value as ClarityValue},
};
use frost_signer::config::{PublicKeys, SignerKeyIds};
use reqwest::{
    blocking::{Client, Response},
    StatusCode,
};
use serde_json::{json, Value};
use tracing::{debug, warn};
use wsts::ecdsa::PublicKey;

pub struct NodeClient {
    node_url: String,
    client: Client,
    contract_name: ContractName,
    contract_address: StacksAddress,
    next_nonce: Option<u64>,
}

impl NodeClient {
    pub fn new(
        node_url: String,
        contract_name: ContractName,
        contract_address: StacksAddress,
    ) -> Self {
        Self {
            node_url,
            client: Client::new(),
            contract_name,
            contract_address,
            next_nonce: None,
        }
    }

    fn build_url(&self, route: &str) -> String {
        format!("{}{}", self.node_url, route)
    }

    fn get_response(&self, route: &str) -> Result<Response, StacksNodeError> {
        let url = self.build_url(route);
        debug!("Sending Request to Stacks Node: {}", &url);
        let now = Instant::now();
        let notify = |_err, dur| {
            debug!("Failed to connect to {}. Next attempt in {:?}", &url, dur);
        };

        let send_request = || {
            if now.elapsed().as_secs() > 5 {
                debug!("Timeout exceeded.");
                return Err(backoff::Error::Permanent(StacksNodeError::Timeout));
            }
            let request = self.client.get(&url);
            let response = request.send().map_err(StacksNodeError::ReqwestError)?;
            Ok(response)
        };
        let backoff_timer = backoff::ExponentialBackoffBuilder::new()
            .with_initial_interval(Duration::from_millis(2))
            .with_max_interval(Duration::from_millis(128))
            .build();

        let response = backoff::retry_notify(backoff_timer, send_request, notify)
            .map_err(|_| StacksNodeError::Timeout)?;

        Ok(response)
    }

    fn get_burn_ops<T>(&self, block_height: u64, op: &str) -> Result<Vec<T>, StacksNodeError>
    where
        T: serde::de::DeserializeOwned,
    {
        let json = self
            .get_response(&format!("/v2/burn_ops/{block_height}/{op}"))?
            .json::<Value>()
            .map_err(|_| StacksNodeError::UnknownBlockHeight(block_height))?;
        Ok(serde_json::from_value(json[op].clone())?)
    }

    fn num_signers(&self, sender: &StacksAddress) -> Result<u128, StacksNodeError> {
        let function_name = "get-num-signers";
        let total_signers_hex = self.call_read(sender, function_name, &[])?;
        let total_signers = ClarityValue::try_deserialize_hex_untyped(&total_signers_hex)?;
        if let ClarityValue::UInt(total_signers) = total_signers {
            Ok(total_signers)
        } else {
            Err(StacksNodeError::MalformedClarityValue(
                function_name.to_string(),
                total_signers,
            ))
        }
    }

    fn signer_data(
        &self,
        sender: &StacksAddress,
        id: u128,
        public_keys: &mut PublicKeys,
        signer_key_ids: &mut SignerKeyIds,
    ) -> Result<(), StacksNodeError> {
        let function_name = "get-signer-data";
        let signer_data_hex = self.call_read(
            sender,
            function_name,
            &[&ClarityValue::UInt(id).to_string()],
        )?;
        let signer_data = ClarityValue::try_deserialize_hex_untyped(&signer_data_hex)?;
        if let ClarityValue::Optional(optional_data) = signer_data.clone() {
            if let Some(ClarityValue::Tuple(tuple_data)) = optional_data.data.map(|boxed| *boxed) {
                let public_key =
                    if let Some(ClarityValue::Sequence(SequenceData::Buffer(public_key))) =
                        tuple_data.data_map.get(&ClarityName::from("public-key"))
                    {
                        PublicKey::try_from(public_key.data.as_slice()).map_err(|_| {
                            StacksNodeError::MalformedClarityValue(
                                function_name.to_string(),
                                signer_data.clone(),
                            )
                        })?
                    } else {
                        return Err(StacksNodeError::MalformedClarityValue(
                            function_name.to_string(),
                            signer_data,
                        ));
                    };
                public_keys
                    .signers
                    .insert(id.try_into().unwrap(), public_key);
                if let Some(ClarityValue::Sequence(SequenceData::List(keys_ids))) =
                    tuple_data.data_map.get(&ClarityName::from("key-ids"))
                {
                    let mut this_signer_key_ids = Vec::new();
                    for key_id in &keys_ids.data {
                        if let ClarityValue::UInt(key_id) = key_id {
                            public_keys
                                .key_ids
                                .insert((*key_id).try_into().unwrap(), public_key);
                            this_signer_key_ids.push((*key_id).try_into().unwrap());
                        } else {
                            return Err(StacksNodeError::MalformedClarityValue(
                                function_name.to_string(),
                                signer_data,
                            ));
                        }
                    }
                    signer_key_ids.insert(id.try_into().unwrap(), this_signer_key_ids);
                }
            } else {
                return Err(StacksNodeError::NoSignerData(id));
            }
        }
        Err(StacksNodeError::MalformedClarityValue(
            function_name.to_string(),
            signer_data,
        ))
    }

    fn call_read(
        &self,
        sender: &StacksAddress,
        function_name: &str,
        function_args: &[&str],
    ) -> Result<String, StacksNodeError> {
        debug!("Calling read-only function {}...", function_name);
        let body = json!({"sender": sender.to_string(), "arguments": function_args}).to_string();
        let url = self.build_url(&format!(
            "/v2/contracts/call-read/{}/{}/{function_name}",
            self.contract_address,
            self.contract_name.as_str()
        ));
        let response = self
            .client
            .post(url)
            .header("content-type", "application/json")
            .body(body)
            .send()?
            .json::<serde_json::Value>()?;
        if !response
            .get("okay")
            .map(|val| val.as_bool().unwrap_or(false))
            .unwrap_or(false)
        {
            let cause = response
                .get("cause")
                .ok_or(StacksNodeError::InvalidJsonEntry("cause".to_string()))?;
            return Err(StacksNodeError::ReadOnlyFailure(format!(
                "{}: {}",
                function_name, cause
            )));
        }
        let result = response
            .get("result")
            .ok_or(StacksNodeError::InvalidJsonEntry("result".to_string()))?
            .as_str()
            .ok_or_else(|| StacksNodeError::ReadOnlyFailure("Expected string result.".to_string()))?
            .to_string();
        Ok(result)
    }
}

impl StacksNode for NodeClient {
    fn get_peg_in_ops(&self, block_height: u64) -> Result<Vec<PegInOp>, StacksNodeError> {
        debug!("Retrieving peg-in ops...");
        self.get_burn_ops::<PegInOp>(block_height, "peg_in")
    }

    fn get_peg_out_request_ops(
        &self,
        block_height: u64,
    ) -> Result<Vec<PegOutRequestOp>, StacksNodeError> {
        debug!("Retrieving peg-out request ops...");
        self.get_burn_ops::<PegOutRequestOp>(block_height, "peg_out_request")
    }

    fn burn_block_height(&self) -> Result<u64, StacksNodeError> {
        debug!("Retrieving burn block height...");
        let json = self.get_response("/v2/info")?.json::<Value>()?;
        let entry = "burn_block_height";
        json[entry]
            .as_u64()
            .ok_or_else(|| StacksNodeError::InvalidJsonEntry(entry.to_string()))
    }

    fn next_nonce(&mut self, address: &StacksAddress) -> Result<u64, StacksNodeError> {
        debug!("Retrieving next nonce...");
        if let Some(nonce) = self.next_nonce {
            let next_nonce = nonce.wrapping_add(1);
            self.next_nonce = Some(next_nonce);
            return Ok(next_nonce);
        }
        let address = address.to_string();
        let entry = "nonce";
        let route = format!("/v2/accounts/{}", address);
        let response = self.get_response(&route)?;
        if response.status() == StatusCode::NOT_FOUND {
            return Err(StacksNodeError::UnknownAddress(address));
        }
        let json = response
            .json::<Value>()
            .map_err(|_| StacksNodeError::BehindChainTip)?;
        let nonce = json
            .get(entry)
            .and_then(|nonce| nonce.as_u64())
            .ok_or_else(|| StacksNodeError::InvalidJsonEntry(entry.to_string()))?;
        self.next_nonce = Some(nonce);
        Ok(nonce)
    }

    fn broadcast_transaction(&self, tx: &StacksTransaction) -> Result<(), StacksNodeError> {
        debug!("Broadcasting transaction...");
        debug!("Transaction: {:?}", tx);
        let url = self.build_url("/v2/transactions");
        let mut buffer = vec![];

        tx.consensus_serialize(&mut buffer)?;

        let response = self
            .client
            .post(url)
            .header("content-type", "application/octet-stream")
            .body(buffer)
            .send()?;

        if response.status() != StatusCode::OK {
            let json_response = response
                .json::<serde_json::Value>()
                .map_err(|_| StacksNodeError::BehindChainTip)?;
            let error_str = json_response
                .get("reason")
                .and_then(|reason| reason.as_str())
                .unwrap_or("Unknown Reason");
            warn!(
                "Failed to broadcast transaction to the stacks node: {:?}",
                error_str
            );
            return Err(StacksNodeError::BroadcastFailure(error_str.to_string()));
        }
        Ok(())
    }

    fn keys_threshold(&self, sender: &StacksAddress) -> Result<u128, StacksNodeError> {
        let function_name = "get-threshold";
        let threshold_hex = self.call_read(sender, function_name, &[])?;
        let threshold = ClarityValue::try_deserialize_hex_untyped(&threshold_hex)?;
        if let ClarityValue::UInt(keys_threshold) = threshold {
            Ok(keys_threshold)
        } else {
            Err(StacksNodeError::MalformedClarityValue(
                function_name.to_string(),
                threshold,
            ))
        }
    }

    fn public_keys(&self, sender: &StacksAddress) -> Result<PublicKeys, StacksNodeError> {
        let total_signers = self.num_signers(sender)?;
        // Retrieve all the signers
        let mut public_keys = PublicKeys::default();
        let mut signer_key_ids = SignerKeyIds::default();
        for id in 1..=total_signers {
            self.signer_data(sender, id, &mut public_keys, &mut signer_key_ids)?;
        }
        Ok(public_keys)
    }

    fn signer_key_ids(&self, sender: &StacksAddress) -> Result<SignerKeyIds, StacksNodeError> {
        let total_signers = self.num_signers(sender)?;
        // Retrieve all the signers
        let mut public_keys = PublicKeys::default();
        let mut signer_key_ids = SignerKeyIds::default();
        for id in 1..=total_signers {
            self.signer_data(sender, id, &mut public_keys, &mut signer_key_ids)?;
        }
        Ok(signer_key_ids)
    }

    fn coordinator_public_key(
        &self,
        sender: &StacksAddress,
    ) -> Result<Option<PublicKey>, StacksNodeError> {
        let function_name = "get-coordinator-data";
        let coordinator_data_hex = self.call_read(sender, function_name, &[])?;
        let coordinator_data = ClarityValue::try_deserialize_hex_untyped(&coordinator_data_hex)?;
        if let ClarityValue::Optional(optional_data) = coordinator_data.clone() {
            if let Some(ClarityValue::Tuple(tuple_data)) = optional_data.data.map(|boxed| *boxed) {
                let value = tuple_data
                    .data_map
                    .get(&ClarityName::from("key"))
                    .ok_or_else(|| {
                        StacksNodeError::MalformedClarityValue(
                            function_name.to_string(),
                            coordinator_data.clone(),
                        )
                    })?;
                if let ClarityValue::Sequence(SequenceData::Buffer(coordinator_public_key)) = value
                {
                    let public_key = PublicKey::try_from(coordinator_public_key.data.as_slice())
                        .map_err(|_| {
                            StacksNodeError::MalformedClarityValue(
                                function_name.to_string(),
                                coordinator_data,
                            )
                        })?;
                    return Ok(Some(public_key));
                } else {
                    return Err(StacksNodeError::MalformedClarityValue(
                        function_name.to_string(),
                        coordinator_data,
                    ));
                }
            }
            Err(StacksNodeError::MalformedClarityValue(
                function_name.to_string(),
                coordinator_data,
            ))
        } else {
            Ok(None)
        }
    }

    fn bitcoin_wallet_public_key(
        &self,
        sender: &StacksAddress,
    ) -> Result<Option<XOnlyPublicKey>, StacksNodeError> {
        let function_name = "get-bitcoin-wallet-public-key";
        let bitcoin_wallet_public_key_hex = self.call_read(sender, function_name, &[])?;
        let bitcoin_wallet_public_key =
            ClarityValue::try_deserialize_hex_untyped(&bitcoin_wallet_public_key_hex)?;
        if let ClarityValue::Optional(optional_data) = bitcoin_wallet_public_key.clone() {
            if let Some(ClarityValue::Sequence(SequenceData::Buffer(public_key))) =
                optional_data.data.map(|boxed| *boxed)
            {
                let public_key =
                    bitcoin::PublicKey::from_slice(&public_key.data).map_err(|_| {
                        StacksNodeError::MalformedClarityValue(
                            function_name.to_string(),
                            bitcoin_wallet_public_key,
                        )
                    })?;
                return Ok(Some(public_key.inner.x_only_public_key().0));
            } else {
                return Ok(None);
            }
        }
        Err(StacksNodeError::MalformedClarityValue(
            function_name.to_string(),
            bitcoin_wallet_public_key,
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::{BufWriter, Read, Write},
        net::{SocketAddr, TcpListener},
        thread::spawn,
    };

    use blockstack_lib::{
        address::{AddressHashMode, C32_ADDRESS_VERSION_TESTNET_SINGLESIG},
        burnchains::Address,
        chainstate::stacks::{
            CoinbasePayload, SinglesigHashMode, SinglesigSpendingCondition, TransactionAnchorMode,
            TransactionAuth, TransactionPayload, TransactionPostConditionMode,
            TransactionPublicKeyEncoding, TransactionSpendingCondition, TransactionVersion,
        },
        types::chainstate::{StacksPrivateKey, StacksPublicKey},
        util::{hash::Hash160, secp256k1::MessageSignature},
    };

    use crate::util::test::PRIVATE_KEY_HEX;

    use super::*;

    struct TestConfig {
        sender: StacksAddress,
        mock_server: TcpListener,
        client: NodeClient,
    }

    impl TestConfig {
        pub fn new() -> Self {
            let sender_key = StacksPrivateKey::from_hex(PRIVATE_KEY_HEX)
                .expect("Unable to generate stacks private key from hex string");

            let pk = StacksPublicKey::from_private(&sender_key);

            let sender = StacksAddress::from_public_keys(
                C32_ADDRESS_VERSION_TESTNET_SINGLESIG,
                &AddressHashMode::SerializeP2PKH,
                1,
                &vec![pk],
            )
            .expect("Failed to generate address from private key");

            let mut mock_server_addr = SocketAddr::from(([127, 0, 0, 1], 0));
            let mock_server = TcpListener::bind(mock_server_addr).unwrap();

            mock_server_addr.set_port(mock_server.local_addr().unwrap().port());
            let client = NodeClient::new(
                format!("http://{}", mock_server_addr),
                ContractName::from("sbtc-alpha"),
                StacksAddress::from_string("SP3FBR2AGK5H9QBDH3EEN6DF8EK8JY7RX8QJ5SVTE").unwrap(),
            );
            Self {
                sender,
                mock_server,
                client,
            }
        }
    }

    fn write_response(mock_server: TcpListener, bytes: &[u8]) -> [u8; 1024] {
        let mut request_bytes = [0u8; 1024];
        {
            let mut stream = mock_server.accept().unwrap().0;

            stream.read(&mut request_bytes).unwrap();
            stream.write(bytes).unwrap();
        }
        request_bytes
    }

    #[test]
    fn call_read_success_test() {
        let config = TestConfig::new();
        let h = spawn(move || {
            config
                .client
                .call_read(&config.sender, "function-name", &[])
        });
        write_response(
            config.mock_server,
            b"HTTP/1.1 200 OK\n\n{\"okay\":true,\"result\":\"0x070d0000000473425443\"}",
        );
        let result = h.join().unwrap().unwrap();
        assert_eq!(result, "0x070d0000000473425443");
    }

    #[test]
    fn call_read_failure_test() {
        let config = TestConfig::new();
        let h = spawn(move || {
            config
                .client
                .call_read(&config.sender, "function-name", &[])
        });
        write_response(
            config.mock_server,
            b"HTTP/1.1 200 OK\n\n{\"okay\":false,\"cause\":\"Some reason\"}",
        );
        let result = h.join().unwrap();
        assert!(matches!(result, Err(StacksNodeError::ReadOnlyFailure(_))));
    }

    #[test]
    fn signer_data_none_test() {
        let config = TestConfig::new();

        let h = spawn(move || {
            let mut public_keys = PublicKeys::default();
            let mut signer_key_ids = SignerKeyIds::default();
            config
                .client
                .signer_data(&config.sender, 1u128, &mut public_keys, &mut signer_key_ids)
        });
        write_response(
            config.mock_server,
            b"HTTP/1.1 200 OK\n\n{\"okay\":true,\"result\":\"0x09\"}",
        );
        let result = h.join().unwrap();
        assert!(matches!(result, Err(StacksNodeError::NoSignerData(_))));
    }

    #[test]
    fn keys_threshold_test() {
        let config = TestConfig::new();

        let h = spawn(move || config.client.keys_threshold(&config.sender));

        write_response(config.mock_server, b"HTTP/1.1 200 OK\n\n{\"okay\":true,\"result\":\"0x0100000000000000000000000000000af0\"}");
        let result = h.join().unwrap().unwrap();
        assert_eq!(result, 2800);
    }

    #[test]
    fn keys_threshold_invalid_test() {
        let config = TestConfig::new();

        let h = spawn(move || config.client.keys_threshold(&config.sender));
        write_response(
            config.mock_server,
            b"HTTP/1.1 200 OK\n\n{\"okay\":true,\"result\":\"0x09\"}",
        );
        let result = h.join().unwrap();
        assert!(matches!(
            result,
            Err(StacksNodeError::MalformedClarityValue(..))
        ));
    }

    #[test]
    fn num_signers_test() {
        let config = TestConfig::new();

        let h = spawn(move || config.client.num_signers(&config.sender));
        write_response(config.mock_server,
                    b"HTTP/1.1 200 OK\n\n{\"okay\":true,\"result\":\"0x0100000000000000000000000000000fa0\"}"
                );
        let result = h.join().unwrap().unwrap();
        assert_eq!(result, 4000);
    }

    #[test]
    fn num_signers_invalid_test() {
        let config = TestConfig::new();

        let h = spawn(move || config.client.num_signers(&config.sender));
        write_response(
            config.mock_server,
            b"HTTP/1.1 200 OK\n\n{\"okay\":true,\"result\":\"0x09\"}",
        );
        let result = h.join().unwrap();
        assert!(matches!(
            result,
            Err(StacksNodeError::MalformedClarityValue(..))
        ));
    }

    #[test]
    fn next_nonce_success_test() {
        let mut config = TestConfig::new();

        let h = spawn(move || {
            let nonce = config.client.next_nonce(&config.sender).unwrap();
            let next_nonce = config.client.next_nonce(&config.sender).unwrap();
            (nonce, next_nonce)
        });
        write_response(config.mock_server,
                    b"HTTP/1.1 200 OK\n\n{\"balance\":\"0x00000000000000000000000000000000\",\"locked\":\"0x00000000000000000000000000000000\",\"unlock_height\":0,\"nonce\":20,\"balance_proof\":\"\",\"nonce_proof\":\"\"}"
                );
        let (nonce, next_nonce) = h.join().unwrap();
        assert_eq!(nonce, 20);
        assert_eq!(next_nonce, 21);
    }

    #[test]
    fn next_nonce_failure_test() {
        let mut config = TestConfig::new();

        let h = spawn(move || config.client.next_nonce(&config.sender));
        write_response(
            config.mock_server,
            b"HTTP/1.1 404 Not Found\n\n/v2/accounts/SP3FBR2AGK5H9QBDH3EEN6DF8EK8JY7RX8QJ5SVTE",
        );
        let result = h.join().unwrap();
        assert!(matches!(result, Err(StacksNodeError::UnknownAddress(_))));
    }

    #[test]
    fn burn_block_height_success_test() {
        let config = TestConfig::new();

        let h = spawn(move || config.client.burn_block_height());
        write_response(
            config.mock_server,
            b"HTTP/1.1 200 OK\n\n{\"peer_version\":420759911,\"burn_block_height\":2430220}",
        );
        let result = h.join().unwrap().unwrap();
        assert_eq!(result, 2430220);
    }

    #[test]
    fn burn_block_height_failure_test() {
        let config = TestConfig::new();

        let h = spawn(move || config.client.burn_block_height());
        write_response(
            config.mock_server,
            b"HTTP/1.1 200 OK\n\n{\"peer_version\":420759911,\"burn_block_height2\":2430220}",
        );
        let result = h.join().unwrap();
        assert!(matches!(result, Err(StacksNodeError::InvalidJsonEntry(_))));
    }

    #[test]
    fn should_send_tx_bytes_to_node() {
        let config = TestConfig::new();
        let tx = StacksTransaction {
            version: TransactionVersion::Testnet,
            chain_id: 0,
            auth: TransactionAuth::Standard(TransactionSpendingCondition::Singlesig(
                SinglesigSpendingCondition {
                    hash_mode: SinglesigHashMode::P2PKH,
                    signer: Hash160([0; 20]),
                    nonce: 0,
                    tx_fee: 0,
                    key_encoding: TransactionPublicKeyEncoding::Uncompressed,
                    signature: MessageSignature([0; 65]),
                },
            )),
            anchor_mode: TransactionAnchorMode::Any,
            post_condition_mode: TransactionPostConditionMode::Allow,
            post_conditions: vec![],
            payload: TransactionPayload::Coinbase(CoinbasePayload([0; 32]), None),
        };

        let mut tx_bytes = [0u8; 1024];
        {
            let mut tx_bytes_writer = BufWriter::new(&mut tx_bytes[..]);
            tx.consensus_serialize(&mut tx_bytes_writer).unwrap();
            tx_bytes_writer.flush().unwrap();
        }

        let bytes_len = tx_bytes
            .iter()
            .enumerate()
            .rev()
            .find(|(_, &x)| x != 0)
            .unwrap()
            .0
            + 1;

        let h = spawn(move || config.client.broadcast_transaction(&tx));

        let request_bytes = write_response(config.mock_server, b"HTTP/1.1 200 OK\n\n");
        h.join().unwrap().unwrap();

        assert!(
            request_bytes
                .windows(bytes_len)
                .any(|window| window == &tx_bytes[..bytes_len]),
            "Request bytes did not contain the transaction bytes"
        );
    }
}
