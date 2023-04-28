(define-constant err-peg-in-expired (err u500))
(define-constant err-not-a-peg-wallet (err u501))

(define-read-only (extract-data (tx (buff 4096)))
	;; It verifies the tapscript is the expected format.
	;; - "before burn height N, address X can spend, or else Y can spend"

	;; Extract data from the Bitcoin transaction/tapscript:
	;; - The total BTC value pegged in, in sats
	;; - The recipient principal as found in the tapscript
	;; - The burnchain peg-in expiry height
	;; make the type checker happy
	(if true (ok {
		recipient: 'ST000000000000000000002AMW42H,
		peg-wallet: { version: 0x00, hashbytes: 0x0011223344556699001122334455669900112233445566990011223344556699},
		value: u100,
		expiry-burn-height: (+ burn-block-height u10)
		})
		(err u1)
		)
)

;; send the mined P2TR spend transaction
;; just some placeholder parameters for now
(define-public (complete-peg-in
	(burn-height uint)
	(tx (buff 4096))
	(header (buff 80))
	(tx-index uint)
	(tree-depth uint)
	(wproof (list 14 (buff 32)))
	(ctx (buff 1024))
	(cproof (list 14 (buff 32)))
	)
	(let (
		;; check if the tx was mined
		(burn-wtxid (try! (contract-call? .clarity-bitcoin was-segwit-tx-mined-compact burn-height tx header tx-index tree-depth wproof 0x 0x ctx cproof)))
		;; extract data from the tx
		(peg-in-data (try! (extract-data tx)))
		)
		;; check if the tx has not been processed before and if the
		;; mined peg-in reached the minimum amount of confirmations.
		(try! (contract-call? .sbtc-registry assert-new-burn-wtxid-and-height burn-wtxid burn-height))
		;; if the transaction is mined before the expiry height, then it means
		;; it was pegged-in. (If after, then it was a reclaim.)
		(asserts! (< burn-height (get expiry-burn-height peg-in-data)) err-peg-in-expired)
		;; check if the recipient is a peg wallet address
		(unwrap! (contract-call? .sbtc-registry get-peg-wallet-cycle (get peg-wallet peg-in-data)) err-not-a-peg-wallet)
		;; mint the tokens
		(contract-call? .sbtc-token protocol-mint (get value peg-in-data) (get recipient peg-in-data))
	)
)
