export class ContractDeployment {
  chainId: string;
  codeId?: number;
  contractAddress?: string;

  constructor(chainId?: string, codeId?: number, contractAddress?: string) {
    this.chainId = chainId!;
    this.codeId = codeId;
    this.contractAddress = contractAddress;
  }
}

export class Contract {
  name: string;
  codeHash?: string;
  deployments?: ContractDeployment[];

  constructor(
    name: string,
    codeHash?: string,
    deployments?: ContractDeployment[]
  ) {
    this.name = name;
    this.codeHash = codeHash;
    this.deployments = deployments;
    this.validate();
  }

  add_code(codeId: number, codeHash: string, chainId: string) {
    this.codeHash = codeHash;
    // add codeId to the matching deployment
    // create the deployment if it is not found
    let deployment = this.deployments!.find((d) => d.chainId === chainId);
    if (deployment === undefined) {
      deployment = new ContractDeployment(chainId, codeId);
      this.deployments!.push(deployment);
    } else {
      deployment.codeId = codeId;
      
    }

    this.validate();
  }

  add_instance(contractAddress: string, chainId: string) {
    // find matching deployment (it must exist) and add contractAddress to it
    let deployment = this.deployments!.find((d) => d.chainId === chainId);
    if (deployment === undefined) {
      throw new Error(`No deployment found for chainId: ${chainId}`);
    }
    deployment.contractAddress = contractAddress;
    this.validate();
  }

  validate(): void {
    if (this.codeHash && !/^[a-fA-F0-9]{64}$/.test(this.codeHash)) {
      throw new Error("Invalid code_hash");
    }
    if (this.deployments) {
      //validate contractAddress to be a valid alphanumeric string beginning with `secret`
      this.deployments.forEach((deployment) => {
        if (
          deployment.contractAddress &&
          !/^secret[a-zA-Z0-9]{39}$/.test(deployment.contractAddress)
        ) {
          throw new Error("Invalid contractAddress");
        }
      });
    }
  }
}

export class ContractDict {
  contracts: Contract[];

  constructor(
    contractList: {
      name: string;
      codeHash?: string;
      deployments?: ContractDeployment[];
    }[]
  ) {
    this.contracts = contractList.map(
      (contract) =>
        new Contract(contract.name, contract.codeHash, contract.deployments)
    );
  }

  getFlattenedAttributes(contractName: string, chainId?: string) {
    const contract = this.contracts.find(
      (contract) => contract.name === contractName
    );
    let deploymentDetails: ContractDeployment | {};
    if (chainId) {
      deploymentDetails = contract?.deployments?.find(
        (deployment) => deployment.chainId === chainId
      ) || {};
    } else {
      deploymentDetails = {};
    }
    return {
      codeHash: contract?.codeHash,
      ...deploymentDetails,
    };
  }
}
