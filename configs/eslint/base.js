// Shared ESLint configuration for ManifestRustTS project
// This can be extended by different parts of the project

import js from '@eslint/js';
import typescript from 'typescript-eslint';
import unicorn from 'eslint-plugin-unicorn';
import importPlugin from 'eslint-plugin-import';
import preferArrow from 'eslint-plugin-prefer-arrow';
import prettier from 'eslint-config-prettier';
import globals from 'globals';

/** @type {import('eslint').Linter.Config[]} */
export const baseConfig = [
  // Base JavaScript configuration
  {
    files: ['**/*.{js,jsx,mjs,cjs,ts,tsx}'],
    languageOptions: {
      ecmaVersion: 2024,
      sourceType: 'module',
      globals: {
        ...globals.es2024,
        ...globals.node
      }
    },
    ...js.configs.recommended
  },

  // TypeScript configuration
  ...typescript.configs.recommendedTypeChecked,

  {
    files: ['**/*.{ts,tsx}'],
    plugins: {
      '@typescript-eslint': typescript.plugin,
      import: importPlugin,
      unicorn,
      'prefer-arrow': preferArrow
    },
    rules: {
      // TypeScript specific rules
      '@typescript-eslint/no-unused-vars': [
        'error',
        { 
          argsIgnorePattern: '^_',
          varsIgnorePattern: '^_',
          caughtErrorsIgnorePattern: '^_'
        }
      ],
      '@typescript-eslint/no-explicit-any': 'warn',
      '@typescript-eslint/explicit-function-return-type': 'off',
      '@typescript-eslint/explicit-module-boundary-types': 'off',
      '@typescript-eslint/no-non-null-assertion': 'warn',
      '@typescript-eslint/prefer-nullish-coalescing': 'error',
      '@typescript-eslint/prefer-optional-chain': 'error',
      '@typescript-eslint/no-unnecessary-type-assertion': 'error',
      '@typescript-eslint/consistent-type-definitions': ['error', 'interface'],
      '@typescript-eslint/consistent-type-imports': [
        'error',
        { prefer: 'type-imports', fixStyle: 'separate-type-imports' }
      ],
      '@typescript-eslint/no-import-type-side-effects': 'error',

      // Import rules
      'import/no-duplicates': 'error',
      'import/no-cycle': 'error',
      'import/no-self-import': 'error',

      // Unicorn rules (modern JavaScript practices)
      'unicorn/no-null': 'off',
      'unicorn/prevent-abbreviations': 'off',
      'unicorn/prefer-top-level-await': 'off',
      'unicorn/no-nested-ternary': 'error',
      'unicorn/prefer-module': 'error',
      'unicorn/prefer-node-protocol': 'error',
      'unicorn/prefer-array-some': 'error',
      'unicorn/prefer-array-find': 'error',
      'unicorn/prefer-string-starts-ends-with': 'error',
      'unicorn/prefer-string-slice': 'error',
      'unicorn/prefer-number-properties': 'error',
      'unicorn/prefer-modern-math-apis': 'error',

      // Prefer arrow functions
      'prefer-arrow/prefer-arrow-functions': [
        'error',
        {
          disallowPrototype: true,
          singleReturnOnly: false,
          classPropertiesAllowed: false
        }
      ],

      // General best practices
      'no-console': ['warn', { allow: ['warn', 'error'] }],
      'no-debugger': 'error',
      'no-alert': 'error',
      'no-var': 'error',
      'prefer-const': 'error',
      'prefer-template': 'error',
      'object-shorthand': 'error',
      'no-duplicate-imports': 'error',
      'no-useless-rename': 'error',
      'no-useless-computed-key': 'error',
      'no-useless-constructor': 'error',
      'no-useless-return': 'error',

      // Performance rules
      'no-loop-func': 'error',
      'no-inner-declarations': 'error',
      'no-implied-eval': 'error',
      'no-new-func': 'error'
    }
  },

  // Prettier compatibility - must be last
  prettier
];

export default baseConfig;
