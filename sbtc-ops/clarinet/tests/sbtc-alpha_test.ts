
import { Clarinet, Tx, Chain, Account, types } from 'https://deno.land/x/clarinet@v1.3.1/index.ts';

Clarinet.test({
    name: "Ensure that coordinator starts empty",
    async fn(chain: Chain, accounts: Map<string, Account>) {
        const deployer = accounts.get("deployer")!;

        let coordinator = chain.callReadOnlyFn("sbtc-alpha", "get-coordinator-data", [], deployer.address);

        coordinator.result.expectNone();
    },
});

Clarinet.test({
    name: "Ensure that coordinator can be written then read",
    async fn(chain: Chain, accounts: Map<string, Account>) {
        const deployer = accounts.get("deployer")!;

        const block = chain.mineBlock([
            Tx.contractCall("sbtc-alpha", "set-coordinator-data", [types.tuple({addr: types.principal(deployer.address), key: types.buff(0x000000000000000000000000000000000000000000000000000000000000000000)})], deployer.address),
        ]);

        const [receipt] = block.receipts;

        receipt.result.expectOk().expectBool(true);

        const coordinator = chain.callReadOnlyFn("sbtc-alpha", "get-coordinator-data", [], deployer.address);

        coordinator.result.expectSome({addr: deployer.address, key: 0x000000000000000000000000000000000000000000000000000000000000000000});
    },
});

Clarinet.test({
    name: "Ensure that num-keys can be written then read",
    async fn(chain: Chain, accounts: Map<string, Account>) {
        const deployer = accounts.get("deployer")!;

        const block = chain.mineBlock([
            Tx.contractCall("sbtc-alpha", "set-num-keys", [types.uint(23)], deployer.address),
        ]);

        const [receipt] = block.receipts;

        receipt.result.expectOk().expectBool(true);

        const coordinator = chain.callReadOnlyFn("sbtc-alpha", "get-num-keys", [], deployer.address);

        coordinator.result.expectUint(23);
    },
});

Clarinet.test({
    name: "Ensure that num-signers can be written then read",
    async fn(chain: Chain, accounts: Map<string, Account>) {
        const deployer = accounts.get("deployer")!;

        const block = chain.mineBlock([
            Tx.contractCall("sbtc-alpha", "set-num-signers", [types.uint(23)], deployer.address),
        ]);

        const [receipt] = block.receipts;

        receipt.result.expectOk().expectBool(true);

        const coordinator = chain.callReadOnlyFn("sbtc-alpha", "get-num-signers", [], deployer.address);

        coordinator.result.expectUint(23);
    },
});

Clarinet.test({
    name: "Ensure that threshold can be written then read",
    async fn(chain: Chain, accounts: Map<string, Account>) {
        const deployer = accounts.get("deployer")!;

        const block = chain.mineBlock([
            Tx.contractCall("sbtc-alpha", "set-threshold", [types.uint(23)], deployer.address),
        ]);

        const [receipt] = block.receipts;

        receipt.result.expectOk().expectBool(true);

        const coordinator = chain.callReadOnlyFn("sbtc-alpha", "get-threshold", [], deployer.address);

        coordinator.result.expectUint(23);
    },
});

Clarinet.test({
    name: "Ensure that bitcoin-wallet-address can be written then read",
    async fn(chain: Chain, accounts: Map<string, Account>) {
        const deployer = accounts.get("deployer")!;

        let block = chain.mineBlock([
            Tx.contractCall("sbtc-alpha", "set-coordinator-data", [types.tuple({addr: types.principal(deployer.address), key: types.buff(0x000000000000000000000000000000000000000000000000000000000000000000)})], deployer.address),
        ]);

        let [receipt] = block.receipts;

        receipt.result.expectOk().expectBool(true);

        block = chain.mineBlock([
            Tx.contractCall("sbtc-alpha", "set-bitcoin-wallet-address", [types.ascii("123456780abcdefghijklmnopqrstuvwxyz")], deployer.address),
        ]);

        [receipt] = block.receipts;

        receipt.result.expectOk();

        const coordinator = chain.callReadOnlyFn("sbtc-alpha", "get-bitcoin-wallet-address", [], deployer.address);

        coordinator.result.expectSome().expectAscii("123456780abcdefghijklmnopqrstuvwxyz");
    },
});

Clarinet.test({
    name: "Ensure that signer can be written then read",
    async fn(chain: Chain, accounts: Map<string, Account>) {
        const deployer = accounts.get("deployer")!;

        const block = chain.mineBlock([
            Tx.contractCall("sbtc-alpha", "set-signer-data", [types.uint(1), types.tuple({addr: types.principal(deployer.address), "public-key": types.buff(0x000000000000000000000000000000000000000000000000000000000000000000), "key-ids": types.list([types.uint(1)])})], deployer.address),
        ]);

        const [receipt] = block.receipts;

        receipt.result.expectOk();

        const coordinator = chain.callReadOnlyFn("sbtc-alpha", "get-signer-data", [types.uint(1)], deployer.address);

        coordinator.result.expectSome(deployer.address);
    },
});

Clarinet.test({
    name: "Ensure that signer can be written then deleted",
    async fn(chain: Chain, accounts: Map<string, Account>) {
        const deployer = accounts.get("deployer")!;

        const block_set = chain.mineBlock([
            Tx.contractCall("sbtc-alpha", "set-signer-data", [types.uint(1), types.tuple({addr: types.principal(deployer.address), "public-key": types.buff(0x000000000000000000000000000000000000000000000000000000000000000000), "key-ids": types.list([types.uint(1)])})], deployer.address),
        ]);

        const [receipt_set] = block_set.receipts;

        receipt_set.result.expectOk();

        const coordinator_set = chain.callReadOnlyFn("sbtc-alpha", "get-signer-data", [types.uint(1)], deployer.address);

        coordinator_set.result.expectSome(deployer.address);

        const block_delete = chain.mineBlock([
            Tx.contractCall("sbtc-alpha", "delete-signer-data", [types.uint(1)], deployer.address),
        ]);

        const [receipt_delete] = block_delete.receipts;

        receipt_delete.result.expectOk();

        const coordinator_delete = chain.callReadOnlyFn("sbtc-alpha", "get-signer-data", [types.uint(1)], deployer.address);

        coordinator_delete.result.expectNone();
    },
});

Clarinet.test({
    name: "Ensure that set-signer-info fails if the signer-id is out of range",
    async fn(chain: Chain, accounts: Map<string, Account>) {
        const deployer = accounts.get("deployer")!;

        let block = chain.mineBlock([
            Tx.contractCall("sbtc-alpha", "set-num-signers", [types.uint(23)], deployer.address),
        ]);

        let [receipt] = block.receipts;

        receipt.result.expectOk().expectBool(true);

        const coordinator = chain.callReadOnlyFn("sbtc-alpha", "get-num-signers", [], deployer.address);

        coordinator.result.expectUint(23);

        block = chain.mineBlock([
            Tx.contractCall("sbtc-alpha", "set-signer-data", [types.uint(23), types.tuple({addr: types.principal(deployer.address), "public-key": types.buff(0x000000000000000000000000000000000000000000000000000000000000000000), "key-ids": types.list([types.uint(1)])})], deployer.address),
        ]);

        [receipt] = block.receipts;

        receipt.result.expectErr();

    },
});

Clarinet.test({
    name: "Ensure we can mint tokens",
    async fn(chain: Chain, accounts: Map<string, Account>) {
        const deployer = accounts.get("deployer")!;
        const alice = accounts.get("wallet_1")!;

        let balance = chain.callReadOnlyFn("sbtc-alpha", "get-balance", [types.principal(alice.address)], alice.address);

        balance.result.expectOk().expectUint(0);

        let block = chain.mineBlock([
            Tx.contractCall("sbtc-alpha", "set-coordinator-data", [types.tuple({addr: types.principal(deployer.address), key: types.buff(0x000000000000000000000000000000000000000000000000000000000000000000)})], deployer.address),
        ]);

        let [receipt] = block.receipts;

        receipt.result.expectOk().expectBool(true);

        block = chain.mineBlock([
            Tx.contractCall("sbtc-alpha", "mint!", [types.uint(1234), types.principal(alice.address), types.ascii("memo")], deployer.address),
        ]);

        [receipt] = block.receipts;

        receipt.result.expectOk().expectBool(true);

        balance = chain.callReadOnlyFn("sbtc-alpha", "get-balance", [types.principal(alice.address)], alice.address);

        balance.result.expectOk().expectUint(1234);
    },
});

Clarinet.test({
    name: "Ensure we can transfer tokens",
    async fn(chain: Chain, accounts: Map<string, Account>) {
        const deployer = accounts.get("deployer")!;
        const alice = accounts.get("wallet_1")!;
        const bob = accounts.get("wallet_2")!;

        let balance = chain.callReadOnlyFn("sbtc-alpha", "get-balance", [types.principal(alice.address)], alice.address);

        balance.result.expectOk().expectUint(0);

        let block = chain.mineBlock([
            Tx.contractCall("sbtc-alpha", "set-coordinator-data", [types.tuple({addr: types.principal(deployer.address), key: types.buff(0x000000000000000000000000000000000000000000000000000000000000000000)})], deployer.address),
        ]);

        let [receipt] = block.receipts;

        receipt.result.expectOk().expectBool(true);

        block = chain.mineBlock([
            Tx.contractCall("sbtc-alpha", "mint!", [types.uint(1234), types.principal(alice.address), types.ascii("memo")], deployer.address),
        ]);

        [receipt] = block.receipts;

        receipt.result.expectOk().expectBool(true);

        balance = chain.callReadOnlyFn("sbtc-alpha", "get-balance", [types.principal(alice.address)], alice.address);

        balance.result.expectOk().expectUint(1234);

        block = chain.mineBlock([
            Tx.contractCall("sbtc-alpha", "transfer", [types.uint(4), types.principal(alice.address), types.principal(bob.address), types.some(types.buff("memo"))], alice.address),
        ]);

        [receipt] = block.receipts;

        receipt.result.expectOk().expectBool(true);

        balance = chain.callReadOnlyFn("sbtc-alpha", "get-balance", [types.principal(alice.address)], alice.address);

        balance.result.expectOk().expectUint(1230);

        balance = chain.callReadOnlyFn("sbtc-alpha", "get-balance", [types.principal(bob.address)], bob.address);

        balance.result.expectOk().expectUint(4);
    },
});

Clarinet.test({
    name: "Ensure we can burn tokens",
    async fn(chain: Chain, accounts: Map<string, Account>) {
        const deployer = accounts.get("deployer")!;
        const alice = accounts.get("wallet_1")!;

        let balance = chain.callReadOnlyFn("sbtc-alpha", "get-balance", [types.principal(alice.address)], alice.address);

        balance.result.expectOk().expectUint(0);

        let block = chain.mineBlock([
            Tx.contractCall("sbtc-alpha", "set-coordinator-data", [types.tuple({addr: types.principal(deployer.address), key: types.buff(0x000000000000000000000000000000000000000000000000000000000000000000)})], deployer.address),
        ]);

        let [receipt] = block.receipts;

        receipt.result.expectOk().expectBool(true);

        block = chain.mineBlock([
            Tx.contractCall("sbtc-alpha", "mint!", [types.uint(1234), types.principal(alice.address), types.ascii("memo")], deployer.address),
        ]);

        [receipt] = block.receipts;

        receipt.result.expectOk().expectBool(true);

        balance = chain.callReadOnlyFn("sbtc-alpha", "get-balance", [types.principal(alice.address)], alice.address);

        balance.result.expectOk().expectUint(1234);

        block = chain.mineBlock([
            Tx.contractCall("sbtc-alpha", "burn!", [types.uint(4), types.principal(alice.address), types.ascii("memo")], deployer.address),
        ]);

        [receipt] = block.receipts;

        receipt.result.expectOk().expectBool(true);

        balance = chain.callReadOnlyFn("sbtc-alpha", "get-balance", [types.principal(alice.address)], alice.address);

        balance.result.expectOk().expectUint(1230);
    },
});

Clarinet.test({
    name: "Ensure burning tokens fails if insufficient balance",
    async fn(chain: Chain, accounts: Map<string, Account>) {
        const deployer = accounts.get("deployer")!;
        const alice = accounts.get("wallet_1")!;

        let balance = chain.callReadOnlyFn("sbtc-alpha", "get-balance", [types.principal(alice.address)], alice.address);

        balance.result.expectOk().expectUint(0);

        let block = chain.mineBlock([
            Tx.contractCall("sbtc-alpha", "set-coordinator-data", [types.tuple({addr: types.principal(deployer.address), key: types.buff(0x000000000000000000000000000000000000000000000000000000000000000000)})], deployer.address),
        ]);

        let [receipt] = block.receipts;

        receipt.result.expectOk().expectBool(true);

        block = chain.mineBlock([
            Tx.contractCall("sbtc-alpha", "mint!", [types.uint(1234), types.principal(alice.address), types.ascii("memo")], deployer.address),
        ]);

        [receipt] = block.receipts;

        receipt.result.expectOk().expectBool(true);

        balance = chain.callReadOnlyFn("sbtc-alpha", "get-balance", [types.principal(alice.address)], alice.address);

        balance.result.expectOk().expectUint(1234);

        block = chain.mineBlock([
            Tx.contractCall("sbtc-alpha", "burn!", [types.uint(1235), types.principal(alice.address), types.ascii("memo")], deployer.address),
        ]);

        [receipt] = block.receipts;

        receipt.result.expectErr();

        balance = chain.callReadOnlyFn("sbtc-alpha", "get-balance", [types.principal(alice.address)], alice.address);

        balance.result.expectOk().expectUint(1234);
    },
});

Clarinet.test({
    name: "Ensure trading-halted can be written then read",
    async fn(chain: Chain, accounts: Map<string, Account>) {
        const deployer = accounts.get("deployer")!;

        let trading_halted = chain.callReadOnlyFn("sbtc-alpha", "get-trading-halted", [], deployer.address);

        trading_halted.result.expectBool(false);

        let block = chain.mineBlock([
            Tx.contractCall("sbtc-alpha", "set-coordinator-data", [types.tuple({addr: types.principal(deployer.address), key: types.buff(0x000000000000000000000000000000000000000000000000000000000000000000)})], deployer.address),
        ]);

        let [receipt] = block.receipts;

        receipt.result.expectOk().expectBool(true);

        block = chain.mineBlock([
            Tx.contractCall("sbtc-alpha", "set-trading-halted", [types.bool(true)], deployer.address),
        ]);

        [receipt] = block.receipts;

        receipt.result.expectOk().expectBool(true);

        trading_halted = chain.callReadOnlyFn("sbtc-alpha", "get-trading-halted", [], deployer.address);

        trading_halted.result.expectBool(true);
    },
});

Clarinet.test({
    name: "Ensure trading can be halted",
    async fn(chain: Chain, accounts: Map<string, Account>) {
        const deployer = accounts.get("deployer")!;
        const alice = accounts.get("wallet_1")!;
        const bob = accounts.get("wallet_2")!;

        let balance = chain.callReadOnlyFn("sbtc-alpha", "get-balance", [types.principal(alice.address)], alice.address);

        balance.result.expectOk().expectUint(0);

        let block = chain.mineBlock([
            Tx.contractCall("sbtc-alpha", "set-coordinator-data", [types.tuple({addr: types.principal(deployer.address), key: types.buff(0x000000000000000000000000000000000000000000000000000000000000000000)})], deployer.address),
        ]);

        let [receipt] = block.receipts;

        receipt.result.expectOk().expectBool(true);

        block = chain.mineBlock([
            Tx.contractCall("sbtc-alpha", "mint!", [types.uint(1234), types.principal(alice.address), types.ascii("memo")], deployer.address),
        ]);

        [receipt] = block.receipts;

        receipt.result.expectOk().expectBool(true);

        balance = chain.callReadOnlyFn("sbtc-alpha", "get-balance", [types.principal(alice.address)], alice.address);

        balance.result.expectOk().expectUint(1234);

        block = chain.mineBlock([
            Tx.contractCall("sbtc-alpha", "transfer", [types.uint(4), types.principal(alice.address), types.principal(bob.address), types.some(types.buff("memo"))], alice.address),
        ]);

        [receipt] = block.receipts;

        receipt.result.expectOk().expectBool(true);

        balance = chain.callReadOnlyFn("sbtc-alpha", "get-balance", [types.principal(alice.address)], alice.address);

        balance.result.expectOk().expectUint(1230);

        balance = chain.callReadOnlyFn("sbtc-alpha", "get-balance", [types.principal(bob.address)], bob.address);

        balance.result.expectOk().expectUint(4);

        block = chain.mineBlock([
            Tx.contractCall("sbtc-alpha", "set-trading-halted", [types.bool(true)], deployer.address),
        ]);

        [receipt] = block.receipts;
        block = chain.mineBlock([
            Tx.contractCall("sbtc-alpha", "transfer", [types.uint(30), types.principal(alice.address), types.principal(bob.address), types.some(types.buff("memo"))], alice.address),
        ]);

        [receipt] = block.receipts;

        receipt.result.expectErr();

        balance = chain.callReadOnlyFn("sbtc-alpha", "get-balance", [types.principal(alice.address)], alice.address);

        balance.result.expectOk().expectUint(1230);

        balance = chain.callReadOnlyFn("sbtc-alpha", "get-balance", [types.principal(bob.address)], bob.address);

        balance.result.expectOk().expectUint(4);
    },
});
