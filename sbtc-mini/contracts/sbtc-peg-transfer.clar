;; sbtc-mini-peg-transfer
;; peg-transfer processor for handing-off pegged-BTC from threshold-wallet (n) to newly-voted-for threshold-wallet (n+1)

;; Handoff Commit/Fund -> On BTC
;; 1. Stackers/signers in cycle/pool N create & fund a Taproot address/script with the current peg-balance that allows for two things:
;;   2. The transaction can be consumed (transferred from wallet n to n+1) a single signature from any of the stackers/signers in cycle/pool N+1
;;   3. The transaction is NOT picked up by the end of the transfer window in n & is reclaimed by the stackers/signers in cycle/pool N

;; Handoff Reveal -> On STX
;; 2. The transaction is consumed & the pegged-BTC is succesfully transferred to the new threshold-wallet (n+1)
;;   2.a. Any observer can verify transfer with a call to .sbtc contracts with the Bitcoin txid of the transfer transaction
;;   This will mark a succesful transfer window & the current pool is moved to the audit/penalty window

;; Handoff Reclaim/Penalty -> On BTC/STX
;; 3. The transaction is NOT consumed & the pegged-BTC is NOT transferred to the new threshold-wallet (n+1)

;; Need a voting receipt  

;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;;;;; Cons, Vars & Maps ;;;;;
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;