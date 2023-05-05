import { Clarinet, Contract, Account } from 'https://deno.land/x/clarinet@v1.5.4/index.ts';

const targetFolder = '.test';

const warningText = `// Code generated using \`clarinet run ./scripts/generate-tests.ts\`
// Manual edits will be lost.`;

function getContractName(contractId: string) {
	return contractId.split('.')[1];
}

function isTestContract(contractName: string) {
	return contractName.substring(contractName.length - 5) === "_test";
}

const functionRegex = /^([ \t]{0,};;[ \t]{0,}@[\s\S]+?)\n[ \t]{0,}\(define-public[\s]+\((.+?)[ \t|)]/gm;
const annotationsRegex = /^;;[ \t]{1,}@([a-z-]+)(?:$|[ \t]+?(.+?))$/;

function extractTestAnnotations(contractSource: string) {
	const functionAnnotations = {};
	const matches = contractSource.replace(/\r/g, "").matchAll(functionRegex);
	for (const [, comments, functionName] of matches) {
		functionAnnotations[functionName] = {};
		const lines = comments.split("\n");
		for (const line of lines) {
			const [, prop, value] = line.match(annotationsRegex) || [];
			if (prop)
				functionAnnotations[functionName][prop] = value ?? true;
		}
	}
	return functionAnnotations;
}

Clarinet.run({
	async fn(accounts: Map<string, Account>, contracts: Map<string, Contract>) {
		Deno.writeTextFile(`${targetFolder}/deps.ts`, generateDeps());

		for (const [contractId, contract] of contracts) {
			const contractName = getContractName(contractId);
			if (!isTestContract(contractName))
				continue;

			const hasDefaultPrepareFunction = contract.contract_interface.functions.reduce(
				(a, v) => a || (v.name === 'prepare' && v.access === 'public' && v.args.length === 0),
				false);
			const annotations = extractTestAnnotations(contract.source);

			const code: string[][] = [];
			code.push([
				warningText,
				``,
				`import { Clarinet, Tx, Chain, Account, types, assertEquals, printEvents, bootstrap } from './deps.ts';`,
				``
			]);

			for (const { name, access, args } of contract.contract_interface.functions.reverse()) {
				if (access !== 'public' || name.substring(0, 5) !== 'test-')
					continue;
				if (args.length > 0)
					throw new Error(`Test functions cannot take arguments. (Offending function: ${name})`);
				const functionAnnotations = annotations[name] || {};
				if (hasDefaultPrepareFunction && !functionAnnotations.prepare)
					functionAnnotations.prepare = 'prepare';
				if (functionAnnotations['no-prepare'])
					delete functionAnnotations.prepare;
				code.push([generateTest(contractId, name, functionAnnotations)]);
			}

			Deno.writeTextFile(`${targetFolder}/${contractName}.ts`, code.flat().join("\n"));
		}
	}
});

type FunctionAnnotations = { [key: string]: string | boolean };

function generatePrepareTx(contractPrincipal: string, annotations: FunctionAnnotations) {
	return `Tx.contractCall('${contractPrincipal}', '${annotations['prepare']}', [], deployer.address)`;
}

function generateNormalMineBlock(contractPrincipal: string, testFunction: string, annotations: FunctionAnnotations) {
	return `let block = chain.mineBlock([
		${annotations['prepare'] ? `${generatePrepareTx(contractPrincipal, annotations)},` : ''}
		Tx.contractCall('${contractPrincipal}', '${testFunction}', [], callerAddress)
	]);`;
}

function generateSpecialMineBlock(mineBlocksBefore: number, contractPrincipal: string, testFunction: string, annotations: FunctionAnnotations) {
	let code = ``;
	if (annotations['prepare']) {
		code = `let prepareBlock = chain.mineBlock([${generatePrepareTx(contractPrincipal, annotations)}]);
		prepareBlock.receipts.map(({result}) => result.expectOk());
		`;
		if (annotations['print'] === 'events')
			code += `\n\t\tprintEvents(prepareBlock);\n`;
	}
	if (mineBlocksBefore > 1)
		code += `
		chain.mineEmptyBlock(${mineBlocksBefore - 1});`;
	return `${code}
		let block = chain.mineBlock([Tx.contractCall('${contractPrincipal}', '${testFunction}', [], callerAddress)]);
		${annotations['print'] === 'events' ? 'printEvents(block);' : ''}`;
}

function generateTest(contractPrincipal: string, testFunction: string, annotations: FunctionAnnotations) {
	const mineBlocksBefore = parseInt(annotations['mine-blocks-before'] as string) || 0;
	return `Clarinet.test({
	name: "${annotations.name ? testFunction + ': ' + (annotations.name as string).replace(/"/g, '\\"') : testFunction}",
	async fn(chain: Chain, accounts: Map<string, Account>) {
		const deployer = accounts.get("deployer")!;
		bootstrap(chain, deployer);
		let callerAddress = ${annotations.caller ? (annotations.caller[0] === "'" ? `"${(annotations.caller as string).substring(1)}"` : `accounts.get('${annotations.caller}')!.address`) : `accounts.get('deployer')!.address`};
		${mineBlocksBefore >= 1
			? generateSpecialMineBlock(mineBlocksBefore, contractPrincipal, testFunction, annotations)
			: generateNormalMineBlock(contractPrincipal, testFunction, annotations)}
		block.receipts.map(({result}) => result.expectOk());
	}
});
`;
}

function generateDeps() {
	return `${warningText}
	
import { Clarinet, Tx, Chain, Account, Block, types } from 'https://deno.land/x/clarinet@v1.5.4/index.ts';
import { assertEquals } from 'https://deno.land/std@0.170.0/testing/asserts.ts';

export { Clarinet, Tx, Chain, types, assertEquals };
export type { Account };

const dirOptions = {strAbbreviateSize: Infinity, depth: Infinity, colors: true};

export function printEvents(block: Block) {
	block.receipts.map(({events}) => events && events.map(event => console.log(Deno.inspect(event, dirOptions))));
}

export const bootstrapContracts = [
	'.sbtc-token',
	'.sbtc-peg-in-processor',
	'.sbtc-peg-out-processor',
	'.sbtc-registry',
	'.sbtc-stacking-pool',
	'.sbtc-testnet-debug-controller',
	'.sbtc-token'
];

export function bootstrap(chain: Chain, deployer: Account) {
	const { receipts } = chain.mineBlock([
		Tx.contractCall(
			\`\${deployer.address}.sbtc-controller\`,
			'upgrade',
			[types.list(bootstrapContracts.map(contract => types.tuple({ contract, enabled: true })))],
			deployer.address
		)
	]);
	receipts[0].result.expectOk().expectList().map(result => result.expectBool(true));
}`;
}