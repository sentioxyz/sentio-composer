import express from 'express';
import { execSync } from 'child_process';
const app = express();

const aptos_bin = "view-function"

app.use(express.json())

interface CallFunctionBody {
  func: string,
  type_params: string,
  params: string,
  ledger_version: number,
  network: string,
}

interface ExecutionResult {
  log_path: String,
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
  if (body.type_params != null && body.type_params.length > 0) {
    command += ` --type_params ${body.type_params}`
  }
  if (body.params != null && body.params.length > 0) {
    command += ` --params ${body.params}`
  }
  if (body.ledger_version != null) {
    command += ` --ledger_version ${body.ledger_version}`
  }
  if (body.network != null) {
    command += ` --network ${body.network}`
  }
  console.log(command);
  const execution_result = execSync(command, {encoding: 'utf-8'});
  console.log(execution_result);
  const parsed_res: ExecutionResult = JSON.parse(execution_result);
  if (parsed_res.return_values != null) {
    res.json({
      details: execution_result,
      error: false
    })
  } else {
    res.json({
      details: execution_result,
      error: true
    })
  }
})  

app.listen(4000, () => {
    console.log('The application is listening on port 4000!');
})
