use crate::error::Result;
use rohas_parser::Schema;
use std::fs;
use std::path::{Path, PathBuf};

pub fn generate_package_json(_schema: &Schema, output_dir: &Path) -> Result<()> {
    let project_root = get_project_root(output_dir);
    let project_name = extract_project_name(&project_root);

    let content = format!(
        r#"{{
  "name": "{}",
  "version": "0.1.0",
  "description": "Rohas event-driven application",
  "main": ".rohas/index.js",
  "type": "module",
  "scripts": {{
    "dev": "rohas dev",
    "build": "npm run compile",
    "compile": "rspack build",
    "compile:watch": "rspack build --watch",
    "start": "node .rohas/index.js",
    "codegen": "rohas codegen",
    "validate": "rohas validate"
  }},
  "dependencies": {{
    "typescript": "^5.3.3"
  }},
  "devDependencies": {{
    "@types/node": "^20.10.0",
    "@rspack/cli": "^1.1.7",
    "@rspack/core": "^1.1.7"
  }},
  "engines": {{
    "node": ">=18.0.0"
  }}
}}
"#,
        project_name
    );

    fs::write(project_root.join("package.json"), content)?;
    Ok(())
}

pub fn generate_tsconfig_json(_schema: &Schema, output_dir: &Path) -> Result<()> {
    let project_root = get_project_root(output_dir);
    let content = r#"{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "node",
    "lib": ["ES2022"],
    "outDir": "./dist",
    "rootDir": "./src",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "resolveJsonModule": true,
    "declaration": true,
    "declarationMap": true,
    "sourceMap": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noImplicitReturns": true,
    "noFallthroughCasesInSwitch": true,
    "baseUrl": ".",
    "paths": {
      "@generated/*": ["src/generated/*"],
      "@handlers/*": ["src/handlers/*"],
      "@/*": ["src/*"]
    }
  },
  "include": [
    "src/**/*"
  ],
  "exclude": [
    "node_modules",
    "dist"
  ]
}
"#;

    fs::write(project_root.join("tsconfig.json"), content)?;
    Ok(())
}

pub fn generate_requirements_txt(_schema: &Schema, output_dir: &Path) -> Result<()> {
    let project_root = get_project_root(output_dir);
    let content = r#"# Python dependencies for Rohas project
# Add your project-specific dependencies here

# Common dependencies
pydantic>=2.0.0
typing-extensions>=4.0.0
"#;

    fs::write(project_root.join("requirements.txt"), content)?;
    Ok(())
}

pub fn generate_pyproject_toml(_schema: &Schema, output_dir: &Path) -> Result<()> {
    let project_root = get_project_root(output_dir);
    let project_name = extract_project_name(&project_root);

    let content = format!(
        r#"[project]
name = "{}"
version = "0.1.0"
description = "Rohas event-driven application"
requires-python = ">=3.9"
dependencies = [
    "pydantic>=2.0.0",
    "typing-extensions>=4.0.0",
]

[project.optional-dependencies]
dev = [
    "pytest>=7.0.0",
    "black>=23.0.0",
    "mypy>=1.0.0",
    "ruff>=0.1.0",
]

[tool.black]
line-length = 100
target-version = ['py39', 'py310', 'py311']

[tool.mypy]
python_version = "3.9"
strict = true
warn_return_any = true
warn_unused_configs = true

[tool.ruff]
line-length = 100
target-version = "py39"
"#,
        project_name
    );

    fs::write(project_root.join("pyproject.toml"), content)?;
    Ok(())
}

pub fn generate_gitignore(_schema: &Schema, output_dir: &Path) -> Result<()> {
    let project_root = get_project_root(output_dir);
    let content = r#"# Dependencies
node_modules/
__pycache__/
*.pyc
*.pyo
*.pyd
.Python
env/
venv/
ENV/
.venv/

# Build outputs
dist/
build/
*.egg-info/
.tsbuildinfo

# IDE
.vscode/
.idea/
*.swp
*.swo
*~
.DS_Store

# Logs
*.log
logs/
npm-debug.log*
yarn-debug.log*
yarn-error.log*

# Environment variables
.env
.env.local
.env.*.local

# OS
.DS_Store
Thumbs.db

# Testing
coverage/
.coverage
.pytest_cache/
*.cover
.hypothesis/

# Rohas compiled output
.rohas/
"#;

    fs::write(project_root.join(".gitignore"), content)?;
    Ok(())
}

pub fn generate_editorconfig(_schema: &Schema, output_dir: &Path) -> Result<()> {
    let project_root = get_project_root(output_dir);
    let content = r#"# EditorConfig is awesome: https://EditorConfig.org

root = true

[*]
charset = utf-8
end_of_line = lf
insert_final_newline = true
trim_trailing_whitespace = true

[*.{ts,tsx,js,jsx,json}]
indent_style = space
indent_size = 2

[*.{py}]
indent_style = space
indent_size = 4

[*.{yml,yaml}]
indent_style = space
indent_size = 2

[*.md]
trim_trailing_whitespace = false
"#;

    fs::write(project_root.join(".editorconfig"), content)?;
    Ok(())
}

pub fn generate_readme(schema: &Schema, output_dir: &Path) -> Result<()> {
    let project_root = get_project_root(output_dir);
    let project_name = extract_project_name(&project_root);
    let has_apis = !schema.apis.is_empty();
    let has_events = !schema.events.is_empty();
    let has_crons = !schema.crons.is_empty();

    let mut api_list = String::new();
    for api in &schema.apis {
        api_list.push_str(&format!("- `{} {}` - {}\n", api.method, api.path, api.name));
    }

    let mut event_list = String::new();
    for event in &schema.events {
        event_list.push_str(&format!(
            "- `{}` - Payload: {}\n",
            event.name, event.payload
        ));
    }

    let mut cron_list = String::new();
    for cron in &schema.crons {
        cron_list.push_str(&format!(
            "- `{}` - Schedule: {}\n",
            cron.name, cron.schedule
        ));
    }

    let content = format!(
        r#"# {}

Rohas event-driven application

## Project Structure

```
├── schema/          # Schema definitions (.roh files)
│   ├── api/        # API endpoint schemas
│   ├── events/     # Event schemas
│   ├── models/     # Data model schemas
│   └── cron/       # Cron job schemas
├── src/
│   ├── generated/  # Auto-generated types (DO NOT EDIT)
│   └── handlers/   # Your handler implementations
│       ├── api/    # API handlers
│       ├── events/ # Event handlers
│       └── cron/   # Cron job handlers
└── config/         # Configuration files
```

## Getting Started

### Installation

```bash
# Install dependencies (TypeScript)
npm install

# Or for Python
pip install -r requirements.txt
```

### Development

```bash
# Generate code from schema
rohas codegen

# Start development server
rohas dev

# Validate schema
rohas validate
```

## Schema Overview

{}{}{}

## Handler Naming Convention

Handler files must be named exactly as the API/Event/Cron name in the schema:

- API `Health` → `src/handlers/api/Health.ts`
- Event `UserCreated` → Handler defined in event schema
- Cron `DailyCleanup` → `src/handlers/cron/DailyCleanup.ts`

## Generated Code

The `src/generated/` directory contains auto-generated TypeScript types and interfaces.
**DO NOT EDIT** these files manually - they will be regenerated when you run `rohas codegen`.

## Adding New Features

1. Define your schema in `schema/` directory
2. Run `rohas codegen` to generate types and handler stubs
3. Implement your handler logic in `src/handlers/`
4. Test with `rohas dev`

## Configuration

See `config/rohas.toml` for project configuration.

## License

MIT
"#,
        project_name,
        if has_apis {
            format!("\n### APIs\n\n{}", api_list)
        } else {
            String::new()
        },
        if has_events {
            format!("\n### Events\n\n{}", event_list)
        } else {
            String::new()
        },
        if has_crons {
            format!("\n### Cron Jobs\n\n{}", cron_list)
        } else {
            String::new()
        },
    );

    let readme_path = project_root.join("README.md");
    if !readme_path.exists() {
        fs::write(readme_path, content)?;
    }

    Ok(())
}

pub fn generate_nvmrc(_schema: &Schema, output_dir: &Path) -> Result<()> {
    let project_root = get_project_root(output_dir);
    let content = "18.0.0\n";
    fs::write(project_root.join(".nvmrc"), content)?;
    Ok(())
}

pub fn generate_prettierrc(_schema: &Schema, output_dir: &Path) -> Result<()> {
    let project_root = get_project_root(output_dir);
    let content = r#"{
  "semi": true,
  "trailingComma": "es5",
  "singleQuote": true,
  "printWidth": 100,
  "tabWidth": 2,
  "useTabs": false,
  "arrowParens": "always"
}
"#;

    fs::write(project_root.join(".prettierrc"), content)?;
    Ok(())
}

pub fn generate_prettierignore(_schema: &Schema, output_dir: &Path) -> Result<()> {
    let project_root = get_project_root(output_dir);
    let content = r#"node_modules/
dist/
build/
coverage/
*.min.js
src/generated/
.rohas/
"#;

    fs::write(project_root.join(".prettierignore"), content)?;
    Ok(())
}

pub fn generate_rspack_config(_schema: &Schema, output_dir: &Path) -> Result<()> {
    let project_root = get_project_root(output_dir);
    let content = r#"const path = require('path');
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
"#;

    fs::write(project_root.join("rspack.config.cjs"), content)?;
    Ok(())
}

fn get_project_root(output_dir: &Path) -> PathBuf {
    if output_dir.file_name().and_then(|s| s.to_str()) == Some("src") {
        output_dir.parent().unwrap_or(output_dir).to_path_buf()
    } else {
        output_dir.to_path_buf()
    }
}

fn extract_project_name(project_root: &Path) -> String {
    project_root
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("rohas-app")
        .to_string()
}
