#[cfg(test)]
mod tests {
    use rand_core::OsRng;
    use wsts::{traits::Signer, v1};

    #[test]
    fn test() {
        let mut rng = OsRng::default();

        let ids = [1, 2, 3];
        let n: u32 = 10;
        let t: u32 = 7;

        let mut signer = v1::Signer::new(1, &ids, n, t, &mut rng);

        assert_eq!(signer.get_key_ids().len(), ids.len());
        signer.gen_nonces(&mut rng);

        let nonces = signer.gen_nonces(&mut rng);
        assert_eq!(nonces.len(), ids.len());
    }
}
