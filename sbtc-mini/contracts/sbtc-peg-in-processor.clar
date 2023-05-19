(define-constant err-peg-in-expired (err u500))
(define-constant err-not-a-peg-wallet (err u501))
(define-constant err-sequence-length-invalid (err u502))
(define-constant err-invalid-principal (err u503))
(define-constant err-op-return-not-found (err u504))
(define-constant err-peg-value-not-found (err u505))
(define-constant err-missing-witness (err u506))
(define-constant err-unlock-script-not-found-or-invalid (err u507))

(define-constant version-P2WPKH 0x04)
(define-constant version-P2WSH 0x05)
(define-constant version-P2TR 0x06)
(define-constant supported-address-versions (list version-P2WPKH version-P2WSH version-P2TR))

(define-constant type-standard-principal 0x05)
(define-constant type-contract-principal 0x06)

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
	(let ((contract-name-length (match (element-at? sequence (+ start u21)) length-byte (buff-to-uint-be length-byte) u0)))
		(from-consensus-buff? principal
			(if (is-eq contract-name-length u0)
				(concat type-standard-principal (try! (slice? sequence start (+ start u21))))
				(concat type-contract-principal (try! (slice? sequence start (+ start u22 contract-name-length))))
			)
		)
	)
)

;; --- Public functions

(define-read-only (prepend-length (input (buff 32)))
	(concat (unwrap-panic (element-at (unwrap-panic (to-consensus-buff? (len input))) u16)) input)
)

;; This function will probably move to a different contract
(define-read-only (peg-wallet-to-scriptpubkey (peg-wallet { version: (buff 1), hashbytes: (buff 32) }))
	(begin
		(asserts! (is-some (index-of? supported-address-versions (get version peg-wallet))) none)
		(some (concat (if (is-eq (get version peg-wallet) version-P2TR) 0x01 0x00) (prepend-length (get hashbytes peg-wallet))))
	)
)

;; (define-read-only (extract-peg-wallet-value (outs (list 8 { value: uint, scriptPubKey: (buff 128) })) (peg-wallet-scriptpubkey (buff 128)))
;; 	(some (+
;; 		(match (element-at? outs u0) out (if (is-eq (get scriptPubKey out) peg-wallet-scriptpubkey) (get value out) u0) u0)
;; 		(match (element-at? outs u1) out (if (is-eq (get scriptPubKey out) peg-wallet-scriptpubkey) (get value out) u0) u0)
;; 		(match (element-at? outs u2) out (if (is-eq (get scriptPubKey out) peg-wallet-scriptpubkey) (get value out) u0) u0)
;; 		(match (element-at? outs u3) out (if (is-eq (get scriptPubKey out) peg-wallet-scriptpubkey) (get value out) u0) u0)
;; 		(match (element-at? outs u4) out (if (is-eq (get scriptPubKey out) peg-wallet-scriptpubkey) (get value out) u0) u0)
;; 		(match (element-at? outs u5) out (if (is-eq (get scriptPubKey out) peg-wallet-scriptpubkey) (get value out) u0) u0)
;; 		(match (element-at? outs u6) out (if (is-eq (get scriptPubKey out) peg-wallet-scriptpubkey) (get value out) u0) u0)
;; 		(match (element-at? outs u7) out (if (is-eq (get scriptPubKey out) peg-wallet-scriptpubkey) (get value out) u0) u0)
;; 	))
;; )

;; TODO: allow to process multiple reveals at once?
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

(define-read-only (get-current-peg-scriptpubkey)
	(peg-wallet-to-scriptpubkey (unwrap! (contract-call? .sbtc-registry get-current-peg-wallet) none))
)

;; send the mined P2TR spend transaction
;; It appears the current wire format of a peg-in is as follows:
;; [op 1 byte] [version 1 byte] [address version 1 byte] [address 20 bytes] [length prefixed contract name] OP_DROP [33 bytes] OP_CHECKSIG
(define-public (complete-peg-in
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
		(vout-value (unwrap! (extract-peg-wallet-vout-value (get outs burn-tx) (unwrap! (get-current-peg-scriptpubkey) err-not-a-peg-wallet)) err-peg-value-not-found))
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
