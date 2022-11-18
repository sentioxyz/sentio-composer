import express from 'express';
import { execFileSync } from 'child_process';
import * as fs from 'fs';
import { HexString } from 'aptos';

const app = express();
const aptos_bin = process.env.BIN_PATH ?? 'bin/view-function';
const SUPPORTED_NETWORK = [
  'testnet',
  'mainnet',
  'devnet'
];
const AccountAddressLength = 32;

app.use(express.json())

interface CallFunctionBody {
  func: string,
  type_args: string,
  args: string,
  ledger_version: number,
  network: string,
}

interface ExecutionResult {
  log_path: string,
  return_values: []
}

app.use(function(req, res, next) {
  res.header("Access-Control-Allow-Origin", "*");
  res.header("Access-Control-Allow-Headers", "Origin, X-Requested-With, Content-Type, Accept");
  next();
});

app.post('/call_function', (req, res) => {
  try {
    let body = req.body as CallFunctionBody
    verify_function_name(body.func);
    let commands = ['--func', `${body.func}`];
    if (body.type_args != null && body.type_args.length > 0) {
      verify_type_args(body.type_args);
      commands = commands.concat('--type_args', `${body.type_args}`);
    }
    if (body.args != null && body.args.length > 0) {
      commands = commands.concat('--args', `${reconstruct_args(body.args)}`);
    }
    if (body.ledger_version != null) {
      verify_ledger_version(body.ledger_version);
      commands = commands.concat('--ledger_version', `${body.ledger_version}`);
    }
    if (body.network != null && body.network.length > 0) {
      verify_network(body.network);
      commands = commands.concat('--network', `${body.network.toLowerCase()}`);
    }
    console.log(commands);
    process.env.RUST_BACKTRACE = '1';
    const execution_result = execFileSync(aptos_bin, commands, {encoding: 'utf-8'});
    console.log(execution_result);
    const parsed_res: ExecutionResult = JSON.parse(execution_result);
    if (parsed_res.return_values != null && parsed_res.return_values.length > 0) {
      res.json({
        details: JSON.stringify(parsed_res.return_values, null, 2),
        error: false
      })
    } else {
      res.json({
        details: read_log(parsed_res.log_path),
        error: true
      })
    }
  } catch (e) {
    //@ts-ignore
    if (e.stderr) {
      res.json({
        //@ts-ignore
        details: e.stderr,
        error: true
      })
    } else {
      res.json({
        //@ts-ignore
        details: (e as Error).message,
        error: true
      })
    }
    throw e;
  }
})

app.listen(4000, () => {
    console.log('The application is listening on port 4000!');
})

function read_log(log_file: string): string {
  return fs.readFileSync(log_file, { encoding: 'utf-8' });
}

function verify_identifier(identifier: string) {
  if (identifier.includes('::')) {
    throw new Error(`Identifier ${identifier} should not contain '::'`);
  }
}

function verify_function_name(qualifiedFuncName: string) {
  const parts = qualifiedFuncName.split("::");
  if (parts.length !== 3) {
    throw new Error("Invalid function name.");
  }
  verify_module_id(`${parts[0]}::${parts[1]}`);
  verify_identifier(parts[2]);
}

function verify_module_id(moduleId: string) {
  const parts = moduleId.split("::");
  if (parts.length !== 2) {
    throw new Error("Invalid module id.");
  }
  verify_account_address(parts[0])
}

function verify_type_args(ty_args: string) {
  const ty_tag_strs = ty_args.split(',');
  ty_tag_strs.forEach(str => verify_struct_tag(str.trim()));
}

function verify_struct_tag(structTag: string) {
   // Type args are not supported in string literal
   if (structTag.includes("<")) {
    throw new Error("Not implemented");
  }

  const parts = structTag.split("::");
  if (parts.length !== 3) {
    throw new Error("Invalid struct tag string literal.");
  }
  verify_account_address(parts[0]);
}

function verify_account_address(account_address: string) {
  let address = HexString.ensure(account_address);

  // If an address hex has odd number of digits, padd the hex string with 0
  // e.g. '1aa' would become '01aa'.
  if (address.noPrefix().length % 2 !== 0) {
    address = new HexString(`0${address.noPrefix()}`);
  }

  const addressBytes = address.toUint8Array();

  if (addressBytes.length > AccountAddressLength) {
    // eslint-disable-next-line quotes
    throw new Error("Hex string is too long. Address's length is 32 bytes.");
  }
}

function reconstruct_args(args: string): string {
  const arg_strs = args.split(',');
  return arg_strs.map(str => str.trim()).join(',');
}

function verify_ledger_version(ledger_version: number) {
  if (ledger_version < 0) {
    throw new Error('Ledger version should be >= 0');
  }
}

function verify_network(network: string) {
  if (!SUPPORTED_NETWORK.includes(network.toLowerCase())) {
    throw new Error(`${network} should be one of ${SUPPORTED_NETWORK}`);
  }
}