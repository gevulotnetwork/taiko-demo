

var ethers = require('ethers');
require('dotenv').config();
const AWS = require('aws-sdk')
const fs = require('fs')


var customHttpProvider = new ethers.JsonRpcProvider(process.env.KATLA_ENDPOINT);



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
    let cmd = `${process.env.PROVER_CMD_PATH} witness_capture -b ${blockNumber} -k ${process.env.PARAMS_PATH} -r ${process.env.KATLA_ENDPOINT} -w witnesses/witness-${blockNumber}.json`
    console.log('  cmd: ', cmd);

    const { stdout, stderr } = await exec(cmd);
    console.log('  stdout:', stdout);
    console.log('  stderr:', stderr);
}

async function calculateChecksum(blockNumber) {
    console.log('callProverCmdCapture ', blockNumber)
    let cmd = `${process.env.GEVULOT_CLI} --jsonurl ${process.env.GEVULOT_JSONURL}  calculate-hash --file witnesses/witness-${blockNumber}.json`
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
    let witness_name = `witness-${blockNumber}.json`
    let srcName = `witnesses/${witness_name}`
    let witness_url = await uploadFile(srcName, witness_name);
    return {witness_checksum, witness_name, witness_url};
}




async function executeProof(witness_checksum, witness_name, witness_url) {
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

async function getVerifierResult(txhash) {
    console.log('\n\n\ngetVerifierResult ', txhash)
    var cmd = `${process.env.GEVULOT_CLI} --jsonurl ${process.env.GEVULOT_JSONURL} print-tx-tree --hash ${txhash}`
    console.log('  cmd: ', cmd);

    var { stdout, stderr } = await exec(cmd);
    console.log('stdout:', stdout);
    console.log('stderr:', stderr);

    let res = stdout.match(/(?<=Leaf: ).*$/gm);
    console.log('res:', res);
    let hash = res[0];
    console.log('hash:', hash);



    var cmd = `${process.env.GEVULOT_CLI} --jsonurl ${process.env.GEVULOT_JSONURL} get-tx-execution-output --hash ${hash}  | jq -r '.[] | select(.kind == "Verification") | .data' | base64 -d`
    // console.log('  cmd: ', cmd);

    var { stdout, stderr } = await exec(cmd);
    console.log('\n\nstdout:', stdout);
    console.log('stderr:', stderr);

    // let res = stdout.match(/(?<=: ).*$/gm);
    // console.log('res:', res);
    return stdout;
}

function getMockWitness() {
    let witness_checksum = '7dacd2a082c5794642d0fba5c68e52e23f3fb423d6e74fe87e27652b5a34f260'
    let witness_name = 'witness-mock.json'
    let witness_url = `https://gevulot.eu-central-1.linodeobjects.com/witness-mock.json`
    return {witness_checksum, witness_name, witness_url};
}

async function writeVerifierResult(verifier_result, filepath) {
    let res = fs.writeFileSync(filepath, verifier_result);
}



async function doBlock(blockNumber) {
    console.log("doBlock ", blockNumber)
    let {witness_checksum, witness_name, witness_url} = await captureWitness(blockNumber)
    // let {witness_checksum, witness_name, witness_url} = getMockWitness()
    console.log(`--${witness_checksum}-- len ${witness_checksum.length}`)
    let txhash = await executeProof(witness_checksum, witness_name, witness_url);
    let verifier_result = getVerifierResult(txhash);
    await writeVerifierResult(verifier_result, `results/result-${blockNumber}`);
}

async function getLatestBlockNumber() {
    let res = await customHttpProvider.getBlockNumber();
    return res;
}



async function doit() {
    let blockNumber = await getLatestBlockNumber()
    console.log('blockNumber: ', blockNumber);
    await doBlock(blockNumber)
}


doit()

