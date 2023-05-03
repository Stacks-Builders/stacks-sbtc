(define-constant mock-wtxid-1 0x000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f)

(define-constant mock-peg-wallet-1 { version: 0x01, hashbytes: 0x0011223344556699001122334455669900112233445566990011223344556699 })
(define-constant mock-peg-wallet-2 { version: 0x01, hashbytes: 0xaa00aa00aa00aa00aa00aa00aa00aa00aa00aa00aa00aa00aa00aa00aa00aa00 })

(define-constant burnchain-confirmations-required u4)

(define-constant err-burn-tx-already-processed (err u600))
(define-constant err-peg-wallet-already-set (err u602))
(define-constant err-minimum-burnchain-confirmations-not-reached (err u603))
(define-constant err-not-settled-state (err u604))
(define-constant err-invalid-txid-length (err u605))
(define-constant err-unknown-peg-out-request (err u606))
(define-constant err-peg-out-not-pending (err u607))

(define-private (assert-eq (result (response bool uint)) (compare (response bool uint)) (message (string-ascii 100)))
	(ok (asserts! (is-eq result compare) (err message)))
)

(define-private (assert-all-eq-iter (item (response bool uint)) (state {compare: (response bool uint), result: bool}))
	{
	compare: (get compare state),
	result: (and (get result state) (is-eq item (get compare state)))
	}
)

(define-private (assert-all-eq (results (list 32 (response bool uint))) (compare (response bool uint)) (message (string-ascii 100)))
	(ok (asserts! (get result (fold assert-all-eq-iter results {compare: compare, result: true})) (err message)))
)

(define-public (prepare)
	(contract-call? .sbtc-testnet-debug-controller set-protocol-contract (as-contract tx-sender) true)
)

;; @name Unique wtxid with the minimum amount of confirmations needed is accepted
;; @mine-blocks-before 5
(define-public (test-assert-wtxid-and-height)
	(begin
		(unwrap!
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height mock-wtxid-1 u1)
			(err "Should have succeeded")
			)
		(ok true)
	)
)

;; @name Unique wtxid without the minimum amount of confirmations needed is NOT accepted
;; @mine-blocks-before 5
(define-public (test-assert-wtxid-and-insufficient-confirmations)
	(assert-eq
		(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height mock-wtxid-1 (- burn-block-height u1))
		err-minimum-burnchain-confirmations-not-reached
		"Should have failed with err-minimum-burnchain-confirmations-not-reached"
	)
)

;; @name A wtxid can only be accepted once
;; @mine-blocks-before 5
(define-public (test-assert-wtxid-uniqueness)
	(begin
		(try! (assert-eq
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height mock-wtxid-1 u1)
			(ok true)
			"Should have succeeded"
			))
		(assert-eq
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height mock-wtxid-1 u1)
			err-burn-tx-already-processed
			"Should have failed with err-burn-tx-already-processed"
			)
	)
)

;; @name wtxid must be 32 bytes
;; @mine-blocks-before 5
(define-public (test-assert-wtxid-length)
	(assert-all-eq
		(list
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x00 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x0000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x00000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x0000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x00000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x0000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x00000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x0000000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x000000000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x00000000000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x0000000000000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x000000000000000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x00000000000000000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x0000000000000000000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x000000000000000000000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x00000000000000000000000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x0000000000000000000000000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x000000000000000000000000000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x00000000000000000000000000000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x0000000000000000000000000000000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x000000000000000000000000000000000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x00000000000000000000000000000000000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x0000000000000000000000000000000000000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x000000000000000000000000000000000000000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x00000000000000000000000000000000000000000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x0000000000000000000000000000000000000000000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x000000000000000000000000000000000000000000000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x00000000000000000000000000000000000000000000000000000000000000 u1)
		)
		err-invalid-txid-length
		"Should have all failed with err-invalid-txid-length"
	)
)