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

app.use(function(req, res, next) {
  res.header("Access-Control-Allow-Origin", "*");
  res.header("Access-Control-Allow-Headers", "Origin, X-Requested-With, Content-Type, Accept");
  next();
});

app.post('/call_function', (req, res) => {
  let body = req.body as CallFunctionBody
  const execution_result = execSync(`${aptos_bin} --func ${body.func} --type_params ${body.type_params} --params ${body.params}`, {encoding: 'utf-8'});
  const lines = execution_result.split('\n')
  if (lines.length > 0) {
    res.json({
      details: lines,
      error: false
    })
  } else {
    res.json({
      details: lines,
      error: true
    })
  }
})  

app.listen(4000, () => {
    console.log('The application is listening on port 4000!');
})
