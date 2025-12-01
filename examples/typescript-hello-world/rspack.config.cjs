const path = require('path');
const fs = require('fs');

// Find all TypeScript handler files
function findHandlers(dir, basePath = '') {
  const entries = {};
  const items = fs.readdirSync(dir, { withFileTypes: true });

  for (const item of items) {
    const fullPath = path.join(dir, item.name);
    const relativePath = path.join(basePath, item.name);

    if (item.isDirectory() && item.name !== 'generated') {
      Object.assign(entries, findHandlers(fullPath, relativePath));
    } else if (item.isFile() && (item.name.endsWith('.ts') || item.name.endsWith('.tsx'))) {
      const entryName = path.join(basePath, item.name.replace(/\.tsx?$/, ''));
      entries[entryName] = fullPath;
    }
  }

  return entries;
}

const srcDir = path.join(__dirname, 'src');
const handlers = findHandlers(srcDir);

/** @type {import('@rspack/cli').Configuration} */
module.exports = {
  mode: 'development',
  entry: handlers,
  output: {
    path: path.resolve(__dirname, '.rohas'),
    filename: '[name].js',
    clean: false,
    library: {
      type: 'commonjs2',
    },
  },
  target: 'node',
  resolve: {
    extensions: ['.ts', '.tsx', '.js', '.jsx'],
    alias: {
      '@generated': path.resolve(__dirname, 'src/generated'),
      '@handlers': path.resolve(__dirname, 'src/handlers'),
      '@': path.resolve(__dirname, 'src'),
    },
  },
  module: {
    rules: [
      {
        test: /\.tsx?$/,
        use: {
          loader: 'builtin:swc-loader',
          options: {
            jsc: {
              parser: {
                syntax: 'typescript',
                tsx: false,
                decorators: true,
                dynamicImport: true,
              },
              target: 'es2022',
              loose: false,
              externalHelpers: false,
              keepClassNames: true,
            },
            module: {
              type: 'commonjs',
            },
          },
        },
        type: 'javascript/auto',
      },
    ],
  },
  externals: [
    // Don't bundle node_modules, treat them as externals
    function ({ request }, callback) {
      // If it's a node module (starts with a letter/@ and not a relative path)
      if (/^[a-z@]/i.test(request)) {
        return callback(null, 'commonjs ' + request);
      }
      callback();
    },
  ],
  devtool: 'source-map',
  optimization: {
    minimize: false,
  },
  stats: {
    preset: 'normal',
    colors: true,
  },
};
