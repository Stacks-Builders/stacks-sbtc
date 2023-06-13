(define-constant err-peg-in-expired (err u500))
(define-constant err-not-a-peg-wallet (err u501))
(define-constant err-invalid-principal (err u503))
(define-constant err-peg-value-not-found (err u505))
(define-constant err-missing-witness (err u506))
(define-constant err-unlock-script-not-found-or-invalid (err u507))

(define-constant type-standard-principal 0x05)
(define-constant type-contract-principal 0x06)

;; --- Public functions

(define-read-only (extract-principal (sequence (buff 128)) (start uint))
	(let ((contract-name-length (match (element-at? sequence (+ start u21)) length-byte (buff-to-uint-be length-byte) u0)))
		(from-consensus-buff? principal
			(if (is-eq contract-name-length u0)
				(concat type-standard-principal (try! (slice? sequence start (+ start u21))))
				(concat type-contract-principal (try! (slice? sequence start (+ start u22 contract-name-length))))
			)
		)
	)
)

;; Bitcoin transactions must only contain one reveal per transaction.
(define-read-only (extract-peg-wallet-vout-value (outs (list 8 { value: uint, scriptPubKey: (buff 128) })) (peg-wallet-scriptpubkey (buff 128)))
	(begin
		(match (element-at? outs u0) out (asserts! (not (is-eq (get scriptPubKey out) peg-wallet-scriptpubkey)) (some {n: u0, value: (get value out)})) false)
		(match (element-at? outs u1) out (asserts! (not (is-eq (get scriptPubKey out) peg-wallet-scriptpubkey)) (some {n: u1, value: (get value out)})) false)
		(match (element-at? outs u2) out (asserts! (not (is-eq (get scriptPubKey out) peg-wallet-scriptpubkey)) (some {n: u2, value: (get value out)})) false)
		(match (element-at? outs u3) out (asserts! (not (is-eq (get scriptPubKey out) peg-wallet-scriptpubkey)) (some {n: u3, value: (get value out)})) false)
		(match (element-at? outs u4) out (asserts! (not (is-eq (get scriptPubKey out) peg-wallet-scriptpubkey)) (some {n: u4, value: (get value out)})) false)
		(match (element-at? outs u5) out (asserts! (not (is-eq (get scriptPubKey out) peg-wallet-scriptpubkey)) (some {n: u5, value: (get value out)})) false)
		(match (element-at? outs u6) out (asserts! (not (is-eq (get scriptPubKey out) peg-wallet-scriptpubkey)) (some {n: u6, value: (get value out)})) false)
		(match (element-at? outs u7) out (asserts! (not (is-eq (get scriptPubKey out) peg-wallet-scriptpubkey)) (some {n: u7, value: (get value out)})) false)
		none
	)
)

(define-public (complete-peg-in
	(cycle uint)
	(burn-height uint)
	(tx (buff 4096))
	(header (buff 80))
	(tx-index uint)
	(tree-depth uint)
	(wproof (list 14 (buff 32)))
	(witness-merkle-root (buff 32))
	(witness-reserved-data (buff 32))
	(ctx (buff 1024))
	(cproof (list 14 (buff 32)))
	)
	(let (
		;; Check if the tx was mined and get the parsed tx.
		(burn-tx (try! (contract-call? .sbtc-btc-tx-helper was-segwit-tx-mined burn-height tx header tx-index tree-depth wproof witness-merkle-root witness-reserved-data ctx cproof)))
		(burn-wtxid (get txid burn-tx))
		;;(value (unwrap! (extract-peg-wallet-value (get outs burn-tx) (unwrap! (get-current-peg-scriptpubkey) err-not-a-peg-wallet)) err-peg-value-not-found))
		;; Extract the vout index and value. (TODO: should get current peg scriptpubkey based on burn height.)
		(vout-value (unwrap! (extract-peg-wallet-vout-value (get outs burn-tx) (unwrap! (contract-call? .sbtc-btc-tx-helper get-peg-wallet-scriptpubkey (some cycle)) err-not-a-peg-wallet)) err-peg-value-not-found))
		;; Find the protocol unlock witness script (TODO: can inline this let var)
		;; It also checks if the protocol opcode and version byte are correct (script must start with 0x3c00).
		(unlock-script (unwrap! (contract-call? .sbtc-btc-tx-helper find-protocol-unlock-witness (unwrap! (element-at? (get witnesses burn-tx) (get n vout-value)) err-missing-witness)) err-unlock-script-not-found-or-invalid))
		;; extract the destination principal from the unlock script
		(recipient (unwrap! (extract-principal unlock-script u3) err-invalid-principal)) ;; skip length byte, protocol opcode, version byte
		(value (get value vout-value))
		)
		;; check if the tx has not been processed before and if the
		;; mined peg-in reached the minimum amount of confirmations.
		(try! (contract-call? .sbtc-registry assert-new-burn-wtxid-and-height burn-wtxid burn-height))
		;; print peg-in event
		(print {event: "peg-in", wtxid: burn-wtxid, value: value, recipient: recipient}) ;; TODO: define protocol events
		;; mint the tokens
		(contract-call? .sbtc-token protocol-mint value recipient)
	)
)
