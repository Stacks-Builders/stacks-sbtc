#[cfg(test)]
mod tests {
    use frost_signer::signing_round::SigningRound;
    use relay_server::Server;
    use yarpc::http::{Call, Method, Request};

    #[test]
    fn template_test() {
        let mut server = Server::default();
        let _signers = [
            SigningRound::new(
                7,
                5,
                10,
                0,
                [0, 1].to_vec(),
                Default::default(),
                Default::default(),
            ),
            SigningRound::new(
                7,
                5,
                10,
                0,
                [2, 3].to_vec(),
                Default::default(),
                Default::default(),
            ),
            SigningRound::new(
                7,
                5,
                10,
                0,
                [4, 5, 6, 7, 8].to_vec(),
                Default::default(),
                Default::default(),
            ),
            SigningRound::new(
                7,
                5,
                10,
                0,
                [10].to_vec(),
                Default::default(),
                Default::default(),
            ),
        ];
        {
            let request = Request::new(
                Method::POST,
                "/".to_string(),
                Default::default(),
                "Hello!".as_bytes().to_vec(),
            );
            let response = server.call(request).unwrap();
            assert_eq!(response.code, 200);
        }
    }
}
