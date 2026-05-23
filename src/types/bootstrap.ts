export type BootstrapState = {
  app: {
    name: string;
    version: string;
    frontend: string;
    backend: string;
  };
  capabilities: string[];
  nextSteps: string[];
};
