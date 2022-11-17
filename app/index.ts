import express from 'express';
import { execFileSync } from 'child_process';
import * as fs from 'fs';
import { ModuleId } from 'aptos/src/aptos_types/transaction';
import { StructTag } from 'aptos/src/aptos_types/type_tag';

const app = express();
const aptos_bin = "bin/view-function"

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
  let body = req.body as CallFunctionBody
  verify_function_name(body.func);
  verify_type_args(body.type_args);
  let commands = ['--func', `${body.func}`];
  if (body.type_args != null && body.type_args.length > 0) {
    commands = commands.concat('--type_args', `${body.type_args}`);
  }
  if (body.args != null && body.args.length > 0) {
    commands = commands.concat('--args', `${reconstruct_args(body.args)}`);
  }
  if (body.ledger_version != null) {
    commands = commands.concat('--ledger_version', `${body.ledger_version}`);
  }
  if (body.network != null && body.network.length > 0) {
    commands = commands.concat('--network', `${body.network}`);
  }
  console.log(commands);
  try {
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
    res.json({
      //@ts-ignore
      details: e.stderr,
      error: true
    })
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
  ModuleId.fromStr(`${parts[0]}::${parts[1]}`);
  verify_identifier(parts[3]);
}

function verify_type_args(ty_args: string) {
  const ty_tag_strs = ty_args.split(',');
  ty_tag_strs.forEach(str => StructTag.fromString(str.trim()));
}

function reconstruct_args(args: string): string {
  const arg_strs = args.split(',');
  return arg_strs.map(str => str.trim()).join(',');
}
