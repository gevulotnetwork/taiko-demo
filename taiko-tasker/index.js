

var ethers = require('ethers');
require('dotenv').config();
const AWS = require('aws-sdk')
const fs = require('fs')


var url = 'http://35.195.113.51:8545';
var customHttpProvider = new ethers.JsonRpcProvider(url);
// customHttpProvider.getBlockNumber().then((result) => {
//     console.log("Current block number: " + result);
// });

var address =  `0xB20BB9105e007Bd3E0F73d63D4D3dA2c8f736b77`;

// var filter = customHttpProvider.filter({fromBlock: 719000, toBlock: 72000, address: "0xB20BB9105e007Bd3E0F73d63D4D3dA2c8f736b77"});
// filter.get(function (err, transactions) {
//   transactions.forEach(function (tx) {
//     var txInfo = customHttpProvider.getTransaction(tx.transactionHash);
//     console.log('txInfo ', txInfo);
//     /* Here you have
//     txInfo.gas;
//     txInfo.from;
//     txInfo.input;
//     */
//   });
// });

// 0x664d8c39e70a87462ff22a8dff5f6a7ccb4a5c422685abf5108a58ddcfab3887


var s3 = new AWS.S3({
    endpoint: 'https://eu-central-1.linodeobjects.com',
    accessKeyId: process.env.BUCKET_ACCESS_KEY,
    secretAccessKey: process.env.BUCKET_SECRET_KEY,
    sslEnabled: true,
  })


async function doTransaction(txhash) {
    let result = await customHttpProvider.getTransaction(txhash);
        if (result.data.length > 2000) {
            //console.log("transaction returned: " + result);
            // console.log("  hash: " + result.hash);
            //console.log("  data: " + result.data);
            // if ()
            console.log(result.data.length, result.hash);
            //console.log("  to: " + result.to);
            //console.log("  from: " + result.from);
            //console.log("  blockHash: " + result.blockHash);
            //console.log("  blockNumber: " + result.blockNumber);

        } else {
            // console.log("  length, block number: " + result.data.length, result.blockNumber);
        }
}

// customHttpProvider.getTransaction
// getHistory(address).then((history) => {
//     history.forEach((tx) => {
//         console.log(tx);
//     })
// });


2

const abi = ["Event proveBlock (uint64, bytes)"]

// const contract = new Contract(address, abi, provider)

// const filter = contract.filters.newProposal(null, null, null)



// async getLogs() {
//     const logs = await customHttpProvider.getLogs({
//       fromBlock: 12794325,
//       toBlock: 'latest',
//       address: this.Dao.address,
//       topics: , //filter
//     });
//     console.log(logs);
//   }


async function doit() {
    var startBlock = 812000;
    const numBlocks = 20;
    while (true) {
        var filter = {
            address: address,
            topics: [],
            fromBlock: startBlock,
            toBlock: startBlock+numBlocks
          };
          
          let logs = await customHttpProvider.getLogs(filter);
          console.log('startBlock ', startBlock);
        //   console.log('logs length ', logs.length);
        //   logs.forEach((log) => {
        //     console.log(log);            
        //     //   if (log.topics && log.topics.length == 2 && log.topics[0] == '0xc195e4be3b936845492b8be4b1cf604db687a4d79ad84d979499c136f8e6701f') {
        //     //     console.log('yes');
        //     //     await doTransaction(log.transactionHash);
        // });
            for (let log of logs) {
            //   console.log(log);
            if (log.topics ) {

            //   console.log('log.topics[0] ', log.topics[0]);
            }
              if (log.topics && log.topics.length == 2 && log.topics[0] == '0xc195e4be3b936845492b8be4b1cf604db687a4d79ad84d979499c136f8e6701f') {
                // console.log('yes');
                await doTransaction(log.transactionHash);
              }
          }
          startBlock -= numBlocks;
        }
}


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
  
    // Uploading files to the bucket
    s3.upload(params, function (err, data) {
      if (err) {
        throw err
      }
      console.log(`File uploaded successfully. ${data.Location}`)
    })
  }
  

// ./target/release/prover_cmd witness_capture -b 57437 
// -k gevulot/kzg_bn254_22.srs 
// -r http://35.205.130.127:8547 -w witness.json
async function callProverCmdCapture(blockNumber) {
    console.log('callProverCmdCapture ', blockNumber)
    let cmd = `${process.env.PROVER_CMD_PATH} witness_capture -b ${blockNumber} -k ${process.env.PARAMS_PATH} -r ${process.env.KATLA_ENDPOINT} -w witnesses/${blockNumber}.json`
    console.log('  cmd: ', cmd);

  const { stdout, stderr } = await exec(cmd);
  console.log('stdout:', stdout);
//   console.log('stderr:', stderr);
}

async function calculateChecksum(blockNumber) {
    console.log('callProverCmdCapture ', blockNumber)
    let cmd = `${process.env.GEVULOT_CLI} calculate-hash --file witnesses/${blockNumber}.json`
    console.log('  cmd: ', cmd);

  const { stdout, stderr } = await exec(cmd);
  console.log('stdout:', stdout);
//   console.log('stderr:', stderr);
//   let text = "The rain in SPAIN stays mainly in the plain";
// str.match(/^([a-z0-9]{5,})$/
  let res = stdout.match(/(?<=: ).*$/gm);
//   const myRe = new RegExp("/g(?<=:).*$");
// const myArray = myRe.exec(stdout);
console.log('res:', res);
return res[0];
}


async function captureWitness(blockNumber) {
    console.log("captureWitness ", blockNumber)
    // await callProverCmdCapture(blockNumber)
    let checksum = await calculateChecksum(blockNumber)
    console.log('  got checksum: ', checksum);
    let srcName = `witnesses/${blockNumber}.json`
    let dstName = `taiko-witness-${blockNumber}.json`
    await uploadFile(srcName, dstName);
}

async function doBlock(blockNumber) {
    console.log("doBlock ", blockNumber)
    await captureWitness(blockNumber)
}

function printEnv() {
    console.log('printEnv');
    console.log('  KATLA_ENDPOINT = ', process.env.KATLA_ENDPOINT);
    console.log('  PARAMS_PATH = ', process.env.PARAMS_PATH);
    console.log('  PROVER_CMD_PATH = ', process.env.PROVER_CMD_PATH);
    console.log('  BUCKET_ACCESS_KEY = ', process.env.BUCKET_ACCESS_KEY);
    console.log('  BUCKET_SECRET_KEY = ', process.env.BUCKET_SECRET_KEY);
}
async function doit2() {
    var startBlock = 57437;
    printEnv();
    await doBlock(startBlock)
}


doit2()

