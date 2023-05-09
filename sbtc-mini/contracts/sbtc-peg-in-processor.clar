(define-constant err-peg-in-expired (err u500))
(define-constant err-not-a-peg-wallet (err u501))
(define-constant err-sequence-length-invalid (err u502))
(define-constant err-stacks-pubkey-invalid (err u503))

(define-data-var minimum-peg-in-amount uint u1000000) ;; 0.01 BTC

(define-read-only (is-protocol-caller (who principal))
	(contract-call? .sbtc-controller is-protocol-caller contract-caller)
)

(define-read-only (get-minimum-peg-in-amount)
	(ok (var-get minimum-peg-in-amount))
)

;; --- Protocol functions

;; #[allow(unchecked_data)]
(define-public (protocol-set-minimum-peg-in-amount (new-minimum uint))
	(begin
		(try! (is-protocol-caller contract-caller))
		(ok (var-set minimum-peg-in-amount new-minimum))
	)
)

(define-read-only (extract-principal (sequence (buff 128)) (start uint))
	(ok (unwrap!
		(principal-of?
			(unwrap-panic (as-max-len? (unwrap! (slice? sequence start (+ start u33)) err-sequence-length-invalid) u33)))
			err-stacks-pubkey-invalid
		))
)

;; --- Public functions

;; It appears the current wire format of a peg-in is as follows:
;; unlock script: [stacks pubkey, 33 bytes] OP_DROP [wallet pubkey, 33 bytes] [p2wpkh pub key, 33 bytes]
;; OP_RETURN    : OP_RETURN [stacks pubkey, 33 bytes]

(define-read-only (extract-data (tx (buff 4096)) (p2tr-unlock-script (buff 128)))
	;; It verifies the tapscript is the expected format.
	;; - "before burn height N, address X can spend, or else Y can spend"

	;; Extract data from the Bitcoin transaction/tapscript:
	;; - The total BTC value pegged in, in sats
	;; - The recipient principal as found in the tapscript
	;; - The burnchain peg-in expiry height

	;; if p2tr-unlock-script is an empty buffer, then the data must be in OP_RETURN.
	;; (is-eq (len p2tr-unlock-script) u0)

	(ok {
		recipient: (try! (extract-principal p2tr-unlock-script u1)), ;; skip length byte
		value: u100,
		expiry-burn-height: (+ burn-block-height u10),
		peg-wallet: { version: 0x01, hashbytes: 0x0011223344556699001122334455669900112233445566990011223344556699}
	})

)

;; send the mined P2TR spend transaction
;; just some placeholder parameters for now
(define-public (complete-peg-in
	(burn-height uint)
	(tx (buff 4096))
	(p2tr-unlock-script (buff 128))
	(header (buff 80))
	(tx-index uint)
	(tree-depth uint)
	(wproof (list 14 (buff 32)))
	(ctx (buff 1024))
	(cproof (list 14 (buff 32)))
	)
	(let (
		;; check if the tx was mined
		(burn-wtxid (try! (contract-call? .clarity-bitcoin was-segwit-tx-mined-compact burn-height tx header tx-index tree-depth wproof ctx cproof)))
		;; extract data from the tx
		(peg-in-data (try! (extract-data tx p2tr-unlock-script)))
		)
		;; check if the tx has not been processed before and if the
		;; mined peg-in reached the minimum amount of confirmations.
		(try! (contract-call? .sbtc-registry assert-new-burn-wtxid-and-height burn-wtxid burn-height))
		;; if the transaction is mined before the expiry height, then it means
		;; it was pegged-in. (If after, then it was a reclaim.)
		(asserts! (< burn-height (get expiry-burn-height peg-in-data)) err-peg-in-expired)
		;; check if the recipient is a peg wallet address
		(unwrap! (contract-call? .sbtc-registry get-peg-wallet-cycle (get peg-wallet peg-in-data)) err-not-a-peg-wallet)
		;; print peg-in event
		(print {event: "peg-in", wtxid: burn-wtxid, data: peg-in-data})
		;; mint the tokens
		(contract-call? .sbtc-token protocol-mint (get value peg-in-data) (get recipient peg-in-data))
	)
)
