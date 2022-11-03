import express from 'express';
import { execSync } from 'child_process';
const app = express();

const aptos_bin = "/Users/poytr1/workspace/move_prototype/target/debug/move_prototype"

app.get('/', (req, res) => {
    res.send('This is a test web page!');
})

app.use(express.json())

interface CallFunctionBody {
  func: string,
  type_params: string,
  params: string,
}

app.post('/call_function', (req, res) => {
  let body = req.body as CallFunctionBody
  const execution_result = execSync(`${aptos_bin} --func ${body.func} --type-params ${body.type_params} --params ${body.params}`, {encoding: 'utf-8'});
  const lines = execution_result.split('\n')
  res.json(lines)
})

app.listen(3000, () => {
    console.log('The application is listening on port 3000!');
})
