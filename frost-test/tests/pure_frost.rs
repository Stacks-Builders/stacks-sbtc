use rand_core::OsRng;
use wsts::bip340::test_helpers::{dkg, sign};
use wsts::bip340::SchnorrProof;
use wsts::v1::{self, SignatureAggregator};

#[test]
#[allow(non_snake_case)]
fn pure_frost_test() {
    let T = 3;
    let N = 4;
    let mut rng = OsRng::default();
    let mut signers = [
        v1::Signer::new(1, &[0, 1], N, T, &mut rng),
        v1::Signer::new(2, &[2], N, T, &mut rng),
        v1::Signer::new(3, &[3], N, T, &mut rng),
    ];

    // DKG (Distributed Key Generation)
    let A = dkg(&mut signers[..], &mut rng).unwrap();

    // signing. Signers: 0 (parties: 0, 1) and 1 (parties: 2)
    let result = {
        // decide which signers will be used
        let mut signers = [signers[0].clone(), signers[1].clone()];

        const MSG: &[u8] = "It was many and many a year ago".as_bytes();

        // get nonces and shares
        let (nonces, shares) = sign(MSG, &mut signers, &mut rng);

        SignatureAggregator::new(N, T, A.clone())
            .unwrap()
            .sign(&MSG, &nonces, &shares)
    };

    assert!(SchnorrProof::new(&result.unwrap()).is_ok());
}
