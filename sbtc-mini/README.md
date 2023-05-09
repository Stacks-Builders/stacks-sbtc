# sBTC Mini protocol

sBTC Mini is a simplified version of the sBTC protocol. It is a work in
progress.

## Bootstrap

The protocol is made up of multiple contracts, each implementing a part of
sBTC Mini. The bootstrapping procedure happens in two steps:

1. Deploy all the contracts in the correct order. Clarinet will determine it for
   you.
2. Send the bootstrapping transaction from the contract deployer.

### Bootstrapping transaction

The first call to the `upgrade` function of the `sbtc-controller` will bootstrap
the protocol. The function can only be called by the contract deployer, and can
only be called once.

The deployer shall provide a list of all sBTC protocol contracts except the
controller itself to enable them all. In a local Clarinet console session, it
can be done as follows:

```clarity
(contract-call? .sbtc-controller upgrade (list {contract: .sbtc-token, enabled: true} {contract: .sbtc-peg-in-processor, enabled: true} {contract: .sbtc-peg-out-processor, enabled: true} {contract: .sbtc-registry, enabled: true} {contract: .sbtc-stacking-pool, enabled: true} {contract: .sbtc-token, enabled: true}))
```

After the bootstrapping transaction is processed, the contract deployer will
have no special access to the protocol and the private key can be discarded or
published.

## Unit testing

### Running tests

All unit tests for sBTC Mini are written in the Clarity language. (As opposed
to TypeScript like is usual for Clarinet projects.) These tests can be found in
the `./tests` folder.

To run all unit tests, invoke the testing script:
```
./scripts/test.sh
```

The test script uses a Clarinet run script to generate unit test stubs for all
test functions in the Clarity unit test contract and will then run those tests.

The purpose of this setup provides the following benefits:

1. The unit tests are written in the same language as the protocol (Clarity.)
2. Test stubs are generated and not checked in, meaning there is one source of
   truth.
3. Using Clarinet allows to make use of its test runner and code coverage report
   generation.

### Adding tests

To write unit tests, follow these steps:

1. Create a new Clarity contract in the `./tests` folder. It can have any name
   but it should end in `_test.clar`. Files that do not follow this convention
   are ignored. (For example: `my-contract_test.clar` will be included and
   `my-contract.clar` will not.)
2. Add the new Clarity contract to `Clarinet.toml`.
3. Write unit tests as public functions, the function name must start with `test-`.
4. Run `./scripts/test.sh` to run the new tests.

### Writing tests

Unit test functions should be public without parameters. If they return an `ok`
response of any kind, the test is considered to have passed whereas an `err`
indicates a failure. The failure value is printed so it can be used to provide a
helpful message. The body of the unit test is written like one would usually
write Clarity, using `try!` and `unwrap!` and so on as needed.

Example:

```clarity
(define-public (test-my-feature)
	(begin
		(unwrap! (contract-call? .some-project-contract my-feature) (err "Calling my-feature failed"))
		(ok true)
	)
)
```

### Prepare function

Sometimes you need to run some preparation logic that is common to all or
multiple unit tests. If the script detects a function called `prepare`, it will
be invoked before calling the unit test function itself. The `prepare` function
should return an `ok`, otherwise the test fails.

```clarity
(define-public (prepare)
	(begin
		(unwrap! (contract-call? .some-project-contract prepare-something) (err "Preparation failed"))
		(ok true)
	)
)

(define-public (test-something)
	;; prepare will be executed before running the test.
)
```

### Annotations

You can add certain comment annotations before unit test functions to add
information or modify behaviour. Annotations are optional.

| Annotation            | Description                                                                                                                                  |
|-----------------------|----------------------------------------------------------------------------------------------------------------------------------------------|
| `@name`               | Give the unit test a name, this text shows up when running unit tests.                                                                       |
| `@no-prepare`         | Do not call the `prepare` function before running this unit test.                                                                            |
| `@prepare`            | Override the default `prepare` function with another. The function name should follow the tag.                                               |
| `@caller`             | Override the default caller when running this unit test. Either specify an account name or standard principal prefixed by a single tick `'`. |
| `@mine-blocks-before` | Mine a number of blocks before running the test. The number of blocks should follow the tag.                                                 |

Examples:

```clarity
(define-public (prepare) (ok "Default prepare function"))

(define-public (custom-prepare) (ok "A custom prepare function"))

;; A test without any annotations
(define-public (test-zero) (ok true))

;; @name A normal test with a name, the prepare function will run before.
(define-public (test-one) (ok true))

;; @name This test will be executed without running the default prepare function.
;; @no-prepare
(define-public (test-two) (ok true))

;; @name Override the default prepare function, it will run custom-prepare instead.
;; @prepare custom-prepare
(define-public (test-three) (ok true))

;; @name This test will be called with tx-sender set to wallet_1 (from the settings toml file).
;; @caller wallet_1
(define-public (test-four) (ok true))

;; @name This test will be called with tx-sender set to the specified principal.
;; @caller 'ST2CY5V39NHDPWSXMW9QDT3HC3GD6Q6XX4CFRK9AG
(define-public (test-five) (ok true))

;; @name Five blocks are mined before this test is executed.
;; @mine-blocks-before 5
(define-public (test-six) (ok true))
```
