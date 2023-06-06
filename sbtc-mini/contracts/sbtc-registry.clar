;; The registry tracks state that should survive a protocol upgrade.
;; It is the data storage component of the controller.

(define-constant peg-out-state-requested 0x00)
(define-constant peg-out-state-fulfilled 0x01)
(define-constant peg-out-state-reclaimed 0x02)

;; Types of penalty errors
(define-constant penalty-unhandled-peg-state-change 0x00)
(define-constant penalty-new-wallet-consensus-failed 0x01)
(define-constant penalty-peg-transfer-failed 0x02)

(define-constant err-burn-tx-already-processed (err u600))
(define-constant err-peg-wallet-already-set (err u602))
(define-constant err-minimum-burnchain-confirmations-not-reached (err u603))
(define-constant err-not-settled-state (err u604))
(define-constant err-invalid-txid-length (err u605))
(define-constant err-unknown-peg-out-request (err u606))
(define-constant err-peg-out-not-pending (err u607))

(define-data-var burnchain-confirmations-required uint u4)
(define-map processed-burn-wtxids (buff 32) bool)

(define-map peg-wallets uint { version: (buff 1), hashbytes: (buff 32) })
(define-map peg-wallets-cycle { version: (buff 1), hashbytes: (buff 32) } uint)
(define-data-var peg-out-request-nonce uint u0)

(define-data-var peg-state bool true)

(define-data-var peg-out-requests-pending uint u0)
(define-map peg-out-requests uint
	{
	value: uint,
	sender: principal,
	destination: { version: (buff 1), hashbytes: (buff 32) },
	unlock-script: (buff 128),
	burn-height: uint,
	expiry-burn-height: uint
	})

(define-map peg-out-request-state uint (buff 1))

(define-read-only (current-peg-state)
	(var-get peg-state)
)

(define-read-only (is-protocol-caller (who principal))
	(contract-call? .sbtc-controller is-protocol-caller contract-caller)
)

(define-read-only (is-burn-wtx-processed (txid (buff 32)))
	(map-get? processed-burn-wtxids txid)
)

(define-public (assert-new-burn-wtxid-and-height (txid (buff 32)) (burn-height uint))
	(begin
		(try! (is-protocol-caller contract-caller))
		(asserts! (is-eq (len txid) u32) err-invalid-txid-length)
		(asserts! (map-insert processed-burn-wtxids txid true) err-burn-tx-already-processed)
		(ok (asserts! (<= (+ burn-height (var-get burnchain-confirmations-required)) burn-block-height) err-minimum-burnchain-confirmations-not-reached))
	)
)

(define-read-only (get-cycle-peg-wallet (cycle uint))
	(map-get? peg-wallets cycle)
)

(define-read-only (get-peg-wallet-cycle (peg-wallet { version: (buff 1), hashbytes: (buff 32) }))
	(map-get? peg-wallets-cycle peg-wallet)
)

(define-read-only (get-current-peg-wallet)
	(map-get? peg-wallets (contract-call? 'ST000000000000000000002AMW42H.pox-2 current-pox-reward-cycle))
)

(define-public (insert-cycle-peg-wallet (cycle uint) (peg-wallet { version: (buff 1), hashbytes: (buff 32) }))
	(begin
		(try! (is-protocol-caller contract-caller))
		(asserts! (map-insert peg-wallets-cycle peg-wallet cycle) err-peg-wallet-already-set)
		(ok (asserts! (map-insert peg-wallets cycle peg-wallet) err-peg-wallet-already-set))
	)
)

(define-read-only (get-peg-out-request (id uint))
	(some (merge
		(try! (map-get? peg-out-requests id))
		{ state: (default-to peg-out-state-requested (map-get? peg-out-request-state id)) }
		)
	)
)

(define-read-only (get-peg-out-request-state (id uint))
	(map-get? peg-out-request-state id)
)

(define-read-only (get-peg-out-nonce)
	(var-get peg-out-request-nonce)
)

(define-read-only (get-pending-wallet-peg-outs)
	(var-get peg-out-requests-pending)
)

;; to-discuss, placeholder for peg-transfer contract
(define-read-only (get-peg-balance)
	u1
)

;; Update peg-state
(define-public (set-peg-state (state bool))
	(begin
		(try! (is-protocol-caller contract-caller))
		(var-set peg-state state)
		(ok state)
	)
)

;; #[allow(unchecked_data)]
(define-public (insert-peg-out-request
	(value uint)
	(sender principal)
	(expiry-burn-height uint)
	(destination { version: (buff 1), hashbytes: (buff 32) })
	(unlock-script (buff 128))
	)
	(let ((nonce (var-get peg-out-request-nonce)))
		(try! (is-protocol-caller contract-caller))
		(map-set peg-out-requests nonce {value: value, sender: sender, destination: destination, unlock-script: unlock-script, burn-height: burn-block-height, expiry-burn-height: expiry-burn-height })
		(var-set peg-out-request-nonce (+ nonce u1))
		(var-set peg-out-requests-pending (+ (var-get peg-out-requests-pending) u1))
		(ok nonce)
	)
)

;; Call and settle a pending peg-out request in one go.
;; It will throw if the peg-out is not pending.
;; It will update the state to a settled state and
;; decrement the pending peg-out request count. Any
;; protocol function calling into this function needs
;; to make sure the transaction reverts if anything
;; goes wrong.
;; #[allow(unchecked_data)]
(define-public (get-and-settle-pending-peg-out-request (id uint) (settled-state (buff 1)))
	(let ((request (unwrap! (map-get? peg-out-requests id) err-unknown-peg-out-request)))
		(try! (is-protocol-caller contract-caller))
		(asserts! (is-eq (default-to peg-out-state-requested (map-get? peg-out-request-state id)) peg-out-state-requested) err-peg-out-not-pending)
		(asserts! (or (is-eq settled-state peg-out-state-fulfilled) (is-eq settled-state peg-out-state-reclaimed)) err-not-settled-state)
		(var-set peg-out-requests-pending (- (var-get peg-out-requests-pending) u1))
		(map-set peg-out-request-state id settled-state)
		(ok request)
	)
)