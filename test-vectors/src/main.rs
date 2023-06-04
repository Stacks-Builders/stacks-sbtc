use bitcoin::consensus::encode::serialize;
use clap::Parser;

fn main() {
    let args = Args::parse();

    let generate_test_vector = match (args.operation, args.protocol) {
        (Operation::PegIn, Protocol::OpReturn) => test_vectors::peg_in::generate_peg_in_test_vector,
        (Operation::PegIn, Protocol::CommitReveal) => {
            test_vectors::peg_in::generate_peg_in_reveal_test_vector
        }
        (Operation::PegOutRequest, Protocol::OpReturn) => {
            test_vectors::peg_out::generate_peg_out_request_test_vector
        }
        (Operation::PegOutRequest, Protocol::CommitReveal) => {
            test_vectors::peg_out::generate_peg_out_request_reveal_test_vector
        }
        (Operation::PegHandoff, Protocol::OpReturn) => {
            test_vectors::peg_handoff::generate_peg_handoff_test_vector
        }
        (Operation::PegHandoff, Protocol::CommitReveal) => {
            unimplemented!()
        }
    };

    let tx = generate_test_vector();
    let hex_tx = array_bytes::bytes2hex("", serialize(&tx));

    println!("{}", hex_tx);
}

/// Generates test vectors for sBTC bitcoin operations
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Which sBTC op to generate for
    #[arg(value_enum)]
    operation: Operation,

    /// Which wire format protocol to use
    #[arg(value_enum)]
    protocol: Protocol,
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
enum Operation {
    PegIn,
    PegOutRequest,
    PegHandoff,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
enum Protocol {
    OpReturn,
    CommitReveal,
}
