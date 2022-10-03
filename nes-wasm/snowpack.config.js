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
  devOptions: {
    hmrDelay: 200,
  }
};
