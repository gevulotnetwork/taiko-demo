

var ethers = require('ethers');
require('dotenv').config();
const AWS = require('aws-sdk')
const fs = require('fs')


var url = 'http://35.195.113.51:8545';
var customHttpProvider = new ethers.JsonRpcProvider(url);
// customHttpProvider.getBlockNumber().then((result) => {
//     console.log("Current block number: " + result);
// });



var s3 = new AWS.S3({
    endpoint: 'https://eu-central-1.linodeobjects.com',
    accessKeyId: process.env.AWS_ACCESS_KEY,
    secretAccessKey: process.env.AWS_SECRET_KEY,
    sslEnabled: true,
  })






const util = require('util');
const exec = util.promisify(require('child_process').exec);


async function uploadFile(srcName, dstName) {
    const fileContent = fs.readFileSync(srcName)
  
    let acl = 'public-read'
    const params = {
      Bucket: 'gevulot',
      Key: dstName,
      ACL: acl,
      Body: fileContent,
    }
    let res = await s3.upload(params).promise();
    console.log('res: ', res);
    return res.Location;
  }
  

async function callProverCmdCapture(blockNumber) {
    console.log('callProverCmdCapture ', blockNumber)
    let cmd = `${process.env.PROVER_CMD_PATH} witness_capture -b ${blockNumber} -k ${process.env.PARAMS_PATH} -r ${process.env.KATLA_ENDPOINT} -w witnesses/${blockNumber}.json`
    console.log('  cmd: ', cmd);

  const { stdout, stderr } = await exec(cmd);
  console.log('  stdout:', stdout);
  console.log('  stderr:', stderr);
}

async function calculateChecksum(blockNumber) {
    console.log('callProverCmdCapture ', blockNumber)
    let cmd = `${process.env.GEVULOT_CLI} calculate-hash --file witnesses/${blockNumber}.json`
    console.log('  cmd: ', cmd);

    const { stdout, stderr } = await exec(cmd);
    console.log('stdout:', stdout);
    let res = stdout.match(/(?<=: ).*$/gm);
    console.log('res:', res);
    return res[0];
}


async function captureWitness(blockNumber) {
    console.log("captureWitness ", blockNumber)
    await callProverCmdCapture(blockNumber)
    let witness_checksum = await calculateChecksum(blockNumber)
    console.log('  got checksum: ', witness_checksum);
    let srcName = `witnesses/${blockNumber}.json`
    let dstName = `witness-${blockNumber}.json`
    let witness_url = await uploadFile(srcName, dstName);
    return {witness_checksum, witness_url};
}




async function executeProof(witness_checksum, witness_url, witness_name) {
    console.log('executeProof ')

    let params = 
    [
        {
            program: process.env.PROVER_HASH,
            cmd_args: [
                {
                    name:"-k",
                    value: process.env.PARAMS_PATH
                },
                {
                    name:"-p",
                    value:"/workspace/proof.json"
                },
                {
                    name:"-w",
                    value: `/workspace/${witness_name}`
                }
            ],
            inputs: [
                {
                    Input: {
                        local_path: witness_checksum,
                        vm_path: `/workspace/${witness_name}`, 
                        file_url: witness_url
                    }
                }
            ]
        },
        {
            program: process.env.VERIFIER_HASH,
            cmd_args: [
                {
                    name:"-p",
                    value:"/workspace/proof.json"
                }
            ],
            inputs: [
                {
                    Output: {
                        source_program: process.env.PROVER_HASH,
                        file_name:"/workspace/proof.json"
                    }
                }
            ]
        }
    ]
    let params_str = JSON.stringify(params);
    console.log('params_str \n', params_str);

    let cmd = `RUST_LOG=trace,gevulot=trace ${process.env.GEVULOT_CLI} --jsonurl ${process.env.GEVULOT_JSONURL}  exec --tasks '${params_str}'`
    console.log('  cmd: ', cmd);

    const { stdout, stderr } = await exec(cmd);
    console.log('stdout:', stdout);
    console.log('stderr:', stderr);
    let res = stdout.match(/(?<=Tx hash:).*$/gm);
    console.log('res:', res);
    return res[0];
}

async function doBlock(blockNumber) {
    console.log("doBlock ", blockNumber)
    let {witness_checksum, witness_url} = await captureWitness(blockNumber)
    console.log(`--${witness_checksum}-- len ${witness_checksum.length}`)
    let witness_name = `witness-${blockNumber}.json`
    let txhash = await executeProof(witness_checksum, witness_url, witness_name);
}


async function doit() {
    var startBlock = 57437;
    await doBlock(startBlock)
}


doit()

