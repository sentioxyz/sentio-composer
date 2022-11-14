import express from 'express';
import { execSync } from 'child_process';
import * as fs from 'fs';
const app = express();
const aptos_bin = "view-function"

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
  let command = `${aptos_bin} --func ${body.func}`
  if (body.type_args != null && body.type_args.length > 0) {
    command += ` --type_args ${body.type_args}`
  }
  if (body.args != null && body.args.length > 0) {
    command += ` --args ${body.args}`
  }
  if (body.ledger_version != null) {
    command += ` --ledger_version ${body.ledger_version}`
  }
  if (body.network != null && body.network.length > 0) {
    command += ` --network ${body.network}`
  }

  console.log(command);
  try {
    process.env.RUST_BACKTRACE = '1';
    const execution_result = execSync(command, {encoding: 'utf-8'});
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
