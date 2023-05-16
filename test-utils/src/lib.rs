use bitcoin::secp256k1::{self, All, Message, Secp256k1, SecretKey};
use bitcoin::util::sighash::SighashCache;
use bitcoin::{
    Address, EcdsaSighashType, Network, PrivateKey, PublicKey, Transaction, TxOut, Txid,
};
use hashbrown::HashMap;
use libc::pid_t;
use rand::distributions::Alphanumeric;
use rand::Rng;
use std::fs::{create_dir, remove_dir_all};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::str::FromStr;
use std::thread::{self, sleep};
use std::time::{Duration, SystemTime};
use ureq::serde::Serialize;
use url::Url;

use ctrlc::Signal;
use nix::sys::signal;
use nix::unistd::Pid;
use ureq::serde_json::Value;
use ureq::{self, json, post};

const BITCOIND_URL: &str = "http://abcd:abcd@localhost";
const MIN_PORT: u16 = 20000;
const MAX_PORT: u16 = 25000;

pub struct Process {
    pub datadir: PathBuf,
    pub child: Child,
}

impl Process {
    pub fn new(cmd: &str, args: &[&str], envs: &HashMap<String, String>) -> Self {
        let mut datadir: PathBuf = PathBuf::from_str("/tmp/").unwrap();
        let tempfile: String = "test_utils_"
            .chars()
            .chain(
                rand::thread_rng()
                    .sample_iter(&Alphanumeric)
                    .take(16)
                    .map(char::from),
            )
            .collect();

        datadir = datadir.join(tempfile);
        create_dir(&datadir).unwrap();

        let child = Self::spawn(cmd, args, envs);

        Process { datadir, child }
    }

    fn spawn(cmd: &str, args: &[&str], envs: &HashMap<String, String>) -> Child {
        let child = Command::new(cmd)
            .envs(envs)
            .args(args)
            .stdout(Stdio::inherit())
            .spawn()
            .unwrap_or_else(|_| panic!("{} failed to start", cmd));

        let pid = child.id() as pid_t;

        // Attempt to set a ctrlc handler if it hasn't been set yet
        let _ = ctrlc::set_handler(move || {
            println!("Killing pid {:?}...", pid);

            signal::kill(Pid::from_raw(pid), Signal::SIGTERM)
                .map_err(|e| println!("Warning: signaling pid {} failed {:?}", pid, e))
                .unwrap();
        });

        child
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        match self.child.kill() {
            Ok(_) => (),
            Err(e) => {
                println!("Failed to kill pid {}: {:?}", self.child.id(), e);
            }
        }
        remove_dir_all(&self.datadir).unwrap();
    }
}

pub struct BitcoinProcess {
    url: Url,
    datadir: PathBuf,
    child: Child,
}

impl BitcoinProcess {
    fn spawn(port: u16, datadir: &Path) -> Child {
        let bitcoind_child = Command::new("bitcoind")
            .arg("-regtest")
            .arg("-bind=0.0.0.0:0")
            .arg("-rpcuser=abcd")
            .arg("-rpcpassword=abcd")
            .arg(format!("-rpcport={}", port))
            .arg(format!("-datadir={}", datadir.to_str().unwrap()))
            .stdout(Stdio::null())
            .spawn()
            .expect("bitcoind failed to start");

        let bitcoind_pid = bitcoind_child.id() as pid_t;

        // Attempt to set a ctrlc handler if it hasn't been set yet
        let _ = ctrlc::set_handler(move || {
            println!("Killing bitcoind pid {:?}...", bitcoind_pid);

            signal::kill(Pid::from_raw(bitcoind_pid), Signal::SIGTERM)
                .map_err(|e| {
                    println!(
                        "Warning: signaling bitcoind {} failed {:?}",
                        bitcoind_pid, e
                    )
                })
                .unwrap();
        });

        bitcoind_child
    }

    pub fn rpc(&self, method: &str, params: impl Serialize) -> Value {
        let rpc = json!({"jsonrpc": "1.0", "id": "tst", "method": method, "params": params});

        match post(self.url.as_str()).send_json(&rpc) {
            Ok(response) => {
                let json = response.into_json::<Value>().unwrap();
                let result = json.as_object().unwrap().get("result").unwrap().clone();

                result
            }
            Err(err) => {
                let err_str = err.to_string();
                let err_obj_opt = match err.into_response() {
                    Some(r) => r.into_json::<Value>().unwrap(),
                    None => json!({ "error": &err_str }),
                };

                println!("{} -> {}", rpc, err_obj_opt);

                err_obj_opt
            }
        }
    }

    fn connectivity_check(&self) -> Result<f32, String> {
        let now = SystemTime::now();

        for _tries in 1..120 {
            let uptime = self.rpc("uptime", ());

            if uptime.is_number() {
                return Ok(now.elapsed().unwrap().as_secs_f32());
            } else {
                thread::sleep(Duration::from_millis(500));
            }
        }

        Err("connection timeout".to_string())
    }

    fn port_is_available(port: u16) -> Option<TcpListener> {
        TcpListener::bind(("127.0.0.1", port)).ok()
    }

    fn find_port() -> Option<u16> {
        (MIN_PORT..=MAX_PORT).find(|port| {
            // Keep the port bound for a short amount of time so other tests can pick different ones
            Self::port_is_available(*port)
                .map(|_listener| {
                    sleep(Duration::from_millis(100));
                    true
                })
                .unwrap_or_default()
        })
    }

    pub fn new() -> Self {
        let mut url: Url = BITCOIND_URL.parse().unwrap();
        url.set_port(Self::find_port()).unwrap();

        let mut datadir: PathBuf = PathBuf::from_str("/tmp/").unwrap();
        let tempfile: String = "bitcoind_test_"
            .chars()
            .chain(
                rand::thread_rng()
                    .sample_iter(&Alphanumeric)
                    .take(16)
                    .map(char::from),
            )
            .collect();

        datadir = datadir.join(tempfile);
        create_dir(&datadir).unwrap();

        let child = Self::spawn(url.port().unwrap(), &datadir);

        let this = Self {
            url,
            datadir,
            child,
        };
        this.connectivity_check().unwrap();

        this
    }

    pub fn url(&self) -> &str {
        self.url.as_str()
    }
}

impl Drop for BitcoinProcess {
    fn drop(&mut self) {
        self.child.kill().unwrap();
        remove_dir_all(&self.datadir).unwrap();
    }
}

pub fn generate_wallet() -> (SecretKey, PrivateKey, PublicKey, Address) {
    let secp = Secp256k1::new();
    let secret_key = SecretKey::new(&mut rand::thread_rng());
    let private_key = PrivateKey::new(secret_key, Network::Regtest);
    let public_key = PublicKey::from_private_key(&secp, &private_key);
    let address = Address::p2wpkh(&public_key, Network::Regtest).unwrap();

    (secret_key, private_key, public_key, address)
}

pub fn mine_and_get_coinbase_txid(btcd: &BitcoinProcess, addr: &Address) -> (Txid, String) {
    let block_id = btcd
        .rpc("generatetoaddress", (100, addr.to_string()))
        .as_array()
        .unwrap()[0]
        .as_str()
        .unwrap()
        .to_string();

    let block = btcd.rpc("getblock", (block_id, 1));
    let blockhash = block.get("hash").unwrap().as_str().unwrap().to_string();

    (
        Txid::from_str(block.get("tx").unwrap().get(0).unwrap().as_str().unwrap()).unwrap(),
        blockhash,
    )
}

pub fn sign_transaction(
    addr: &Address,
    secret_key: &SecretKey,
    public_key: &PublicKey,
    prev_output: &TxOut,
    tx: &mut Transaction,
    secp: &Secp256k1<All>,
) {
    let tx_sighash_pubkey_script = addr.script_pubkey().p2wpkh_script_code().unwrap();
    let mut sighash_cache_peg_in = SighashCache::new(&*tx);

    let tx_sighash = sighash_cache_peg_in
        .segwit_signature_hash(
            0,
            &tx_sighash_pubkey_script,
            prev_output.value,
            EcdsaSighashType::All,
        )
        .unwrap();

    let msg = Message::from_slice(&tx_sighash).unwrap();
    let sig = secp.sign_ecdsa_low_r(&msg, secret_key);
    let secp_public_key_source = secp256k1::PublicKey::from_secret_key(secp, secret_key);

    secp.verify_ecdsa(&msg, &sig, &secp_public_key_source)
        .unwrap();

    tx.input[0]
        .witness
        .push_bitcoin_signature(&sig.serialize_der(), EcdsaSighashType::All);
    tx.input[0]
        .witness
        .push(bitcoin::psbt::serialize::Serialize::serialize(public_key));
}
