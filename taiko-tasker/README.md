# Taiko Prover Task Server

Make sure you are in the `/taiko-tasker` folder.

Copy the `.env.template` file to `.env`.  Edit the parameters as needed.


The Gevulot node requires a public url for any data files which are not statically embedded in the unikernel image. This app uses an S3-compatible bucket to store uploaded files, as well as the AWS SDK to interact with it.

```
S3_ENDPOINT=https://eu-central-1.linodeobjects.com
S3_ACCESS_KEY=access-key
S3_SECRET_KEY=secret-key
```

You will need to edit the relevant code if using a different type of public storage.

To run the `taiko-tasker`:

```
npm i
node index.js
```

The app currently runs in an infinite loop, performing the following steps:

- get latest block number from L2 Katla node (`ethers`)
- create witness (`prover_cmd`, a `zkevm-chain` binary)
- calculate hash (`gevulot-cli`)
- upload to S3 storage (`aws-sdk`)
- launch a prover task (`gevulot-cli exec`)
- poll for completion (`gevulot-cli get-tx-execution-ouput`)
- parse results, write to file
