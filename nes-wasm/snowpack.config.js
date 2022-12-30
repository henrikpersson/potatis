module.exports = {
  mount: {
    www: '/',
  },
  workspaceRoot: '.',
  exclude: [
    '**/*.rs',
    '**/*.lock',
    '**/*.toml',
    '**/*.md',
    '**/target/**',
  ],
  buildOptions: {
    metaUrlPath: 'snowpack',
  },
  devOptions: {
    hmrDelay: 200,
  }
};
