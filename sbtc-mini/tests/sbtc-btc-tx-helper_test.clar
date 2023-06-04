;; @name Extract protocol witness script for peg-out reveal transactions.
(define-public (test-find-peg-out-reveal-protocol-unlock-witness)
  (ok (asserts! (is-eq
      (some 0x183c001a7321b74e2b6a7e949e6c4ad313035b16650950170075200046422d30ec92c568e21be4b9579cfed8e71ba0702122b014755ae0e23e3563ac)
      (contract-call? .sbtc-btc-tx-helper find-protocol-unlock-witness (list 0x000102030405060708090a0b0c0d0e0f000102030405060708090a0b0c0d0e0f000102030405060708090a0b0c0d0e0f000102030405060708090a0b0c0d0e0f 0x183c001a7321b74e2b6a7e949e6c4ad313035b16650950170075200046422d30ec92c568e21be4b9579cfed8e71ba0702122b014755ae0e23e3563ac 0xc01dae61a4a8f841952be3a511502d4f56e889ffa0685aa0098773ea2d4309f62474708f439116be919de13c6d3200d2305fcbdf5a9e7d2c079e85b427bb110e90))
      )
    (err "Could not find the witness (should have returned the second item)")
  ))
)
