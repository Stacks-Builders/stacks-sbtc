mod sync_test;
mod v1;

// https://github.com/Trust-Machines/frost/blob/sbtc/src/v1.rs#L444

#[cfg(test)]
mod tests {
    use hashbrown::HashMap;
    use rand_core::{CryptoRng, OsRng, RngCore};
    use wsts::{
        common::PolyCommitment,
        errors::DkgError,
        v1::{Party, SignatureAggregator},
    };

    #[allow(non_snake_case)]
    fn distribute(
        parties: &mut Vec<Party>,
        A: &Vec<PolyCommitment>,
        // B: &Vec<Vec<PublicNonce>>,
    ) -> Result<(), DkgError> {
        // each party broadcasts their commitments
        // these hashmaps will need to be serialized in tuples w/ the value encrypted
        let mut broadcast_shares = Vec::new();
        for i in 0..parties.len() {
            broadcast_shares.push(parties[i].get_shares());
        }

        // each party collects its shares from the broadcasts
        // maybe this should collect into a hashmap first?
        for i in 0..parties.len() {
            let mut h = HashMap::new();
            for j in 0..parties.len() {
                h.insert(j as u32, broadcast_shares[j][&(i as u32)]);
            }
            //let compute_secret_start = time::Instant::now();
            parties[i].compute_secret(h, &A)?;
            //let compute_secret_time = compute_secret_start.elapsed();
            //total_compute_secret_time += compute_secret_time.as_micros();
        }
        // // each party copies the nonces
        // for i in 0..parties.len() {
        //     parties[i].set_group_nonces(B.clone());
        // }
        Ok(())
    }

    #[allow(non_snake_case)]
    fn select_parties<RNG: RngCore + CryptoRng>(N: u32, T: u32, rng: &mut RNG) -> Vec<u32> {
        let mut indices: Vec<u32> = Vec::new();

        for i in 0..N {
            indices.push(i);
        }

        while indices.len() > usize::try_from(T).unwrap() {
            let i = rng.next_u64() as u32 % u32::try_from(indices.len()).unwrap();
            indices.swap_remove(i as usize);
        }

        indices
    }

    /*
    // There might be a slick one-liner for this?
    fn collect_signatures(
        parties: &Vec<Party>,
        signers: &Vec<u32>,
        nonces: &[PublicNonce],
        msg: &String,
    ) -> Vec<SignatureShare> {
        let mut sigs = Vec::new();
        for i in 0..signers.len() {
            let party = &parties[signers[i]];
            sigs.push(SignatureShare {
                id: party.id.clone(),
                z_i: party.sign(&msg, &signers, nonces),
                public_key: party.public_key.clone(),
            });
        }
        sigs
    }
    */

    #[test]
    #[allow(non_snake_case)]
    fn pure_frost() {
        // let num_nonces = 5;
        let N: u32 = 10;
        let T = (N * 2) / 3;

        let mut rng = OsRng::default();

        //
        let mut parties = (0..N)
            .map(|i| Party::new(i, N, T, &mut rng))
            .collect::<Vec<_>>();
        let _nonces = parties
            .iter_mut()
            .map(|p| p.gen_nonce(&mut rng))
            .collect::<Vec<_>>();
        let commitments = parties
            .iter()
            .map(|p| p.get_poly_commitment(&mut rng))
            .collect::<Vec<_>>();

        distribute(&mut parties, &commitments).unwrap();
        let _sig_agg = SignatureAggregator::new(N, T, commitments).unwrap();

        let num_sigs = 7;
        for _sig_ct in 0..num_sigs {
            let _msg = "It was many and many a year ago".to_string();
            let _signers = select_parties(N, T, &mut rng);
            // let nonce_ctr = sig_agg.get_nonce_ctr();
            /*
            let sig_shares = collect_signatures(&parties, &signers, &nonces, &msg);
            let sig = sig_agg.sign(
                msg.as_bytes(), &nonces, &sig_shares);
            */

            // assert!(sig.verify(&sig_agg.key, &msg));

            // this resets one party's nonces assuming it went down and needed to regenerate
            /*
            if sig_ct == 3 {
                let reset_party = 2;
                println!("Resetting nonce for party {}", reset_party);
                reset_nonce(
                    &mut parties,
                    &mut sig_agg,
                    reset_party,
                    num_nonces,
                    &mut rng,
                );
            }

            if sig_agg.get_nonce_ctr() == num_nonces as u32 {
                println!("Everyone's nonces were refilled.");
                let B: Vec<Vec<PublicNonce>> = parties
                    .iter_mut()
                    .map(|p| p.gen_nonces(num_nonces, &mut rng))
                    .collect();
                for p in &mut parties {
                    p.set_group_nonces(B.clone());
                }
                sig_agg.set_group_nonces(B.clone());
            }
            */
        }
    }
}
