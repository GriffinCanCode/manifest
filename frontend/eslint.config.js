// eslint.config.js - Modern ESLint Flat Config (2025)
import js from '@eslint/js';
import prettier from 'eslint-config-prettier';
import importPlugin from 'eslint-plugin-import';
import jsxA11y from 'eslint-plugin-jsx-a11y';
import preferArrow from 'eslint-plugin-prefer-arrow';
import react from 'eslint-plugin-react';
import reactHooks from 'eslint-plugin-react-hooks';
import reactRefresh from 'eslint-plugin-react-refresh';
import unicorn from 'eslint-plugin-unicorn';
import globals from 'globals';
import typescript from 'typescript-eslint';

/** @type {import('eslint').Linter.Config[]} */
export default [
  // Global ignores - applies to all configs
  {
    ignores: [
      '**/node_modules/**',
      '**/dist/**',
      '**/build/**',
      '**/.vite/**',
      '**/coverage/**',
      '**/*.min.js',
      '**/public/**',
      '**/vite.config.ts',
      '**/tailwind.config.js',
      '**/postcss.config.js',
      '**/*.generated.*',
      '**/.DS_Store',
    ],
  },

  // Base JavaScript configuration
  {
    files: ['**/*.{js,jsx,mjs,cjs}'],
    languageOptions: {
      ecmaVersion: 2024,
      sourceType: 'module',
      globals: {
        ...globals.browser,
        ...globals.es2024,
        ...globals.node,
      },
    },
    ...js.configs.recommended,
  },

  // TypeScript files configuration
  ...typescript.configs.recommendedTypeChecked.map(config => ({
    ...config,
    files: ['**/*.{ts,tsx}'],
  })),

  {
    files: ['**/*.{ts,tsx}'],
    languageOptions: {
      ecmaVersion: 2024,
      sourceType: 'module',
      globals: {
        ...globals.browser,
        ...globals.es2024,
      },
      parser: typescript.parser,
      parserOptions: {
        project: './tsconfig.json',
        tsconfigRootDir: import.meta.dirname,
        ecmaFeatures: {
          jsx: true,
        },
      },
    },
    settings: {
      react: {
        version: 'detect',
      },
    },
    plugins: {
      '@typescript-eslint': typescript.plugin,
      react,
      'react-hooks': reactHooks,
      'react-refresh': reactRefresh,
      'jsx-a11y': jsxA11y,
      import: importPlugin,
      unicorn,
      'prefer-arrow': preferArrow,
    },
    rules: {
      // TypeScript specific rules
      '@typescript-eslint/no-unused-vars': [
        'error',
        {
          argsIgnorePattern: '^_',
          varsIgnorePattern: '^_',
          caughtErrorsIgnorePattern: '^_',
        },
      ],
      '@typescript-eslint/no-explicit-any': 'warn',
      '@typescript-eslint/explicit-function-return-type': 'off',
      '@typescript-eslint/explicit-module-boundary-types': 'off',
      '@typescript-eslint/no-non-null-assertion': 'warn',
      '@typescript-eslint/prefer-nullish-coalescing': 'error',
      '@typescript-eslint/prefer-optional-chain': 'error',
      '@typescript-eslint/no-unnecessary-type-assertion': 'error',
      '@typescript-eslint/no-floating-promises': 'error',
      '@typescript-eslint/await-thenable': 'error',
      '@typescript-eslint/no-misused-promises': 'error',
      '@typescript-eslint/consistent-type-definitions': ['error', 'interface'],
      '@typescript-eslint/consistent-type-imports': [
        'error',
        { prefer: 'type-imports', fixStyle: 'separate-type-imports' },
      ],
      '@typescript-eslint/no-import-type-side-effects': 'error',

      // React specific rules
      'react/react-in-jsx-scope': 'off', // Not needed with React 17+
      'react/prop-types': 'off', // Using TypeScript
      'react/display-name': 'off',
      'react/jsx-uses-react': 'off', // Not needed with React 17+
      'react/jsx-uses-vars': 'error',
      'react/jsx-key': ['error', { checkFragmentShorthand: true }],
      'react/jsx-no-duplicate-props': 'error',
      'react/jsx-no-undef': 'error',
      'react/jsx-pascal-case': 'error',
      'react/no-danger': 'warn',
      'react/no-deprecated': 'error',
      'react/no-direct-mutation-state': 'error',
      'react/no-find-dom-node': 'error',
      'react/no-is-mounted': 'error',
      'react/no-render-return-value': 'error',
      'react/no-string-refs': 'error',
      'react/no-unescaped-entities': 'error',
      'react/no-unknown-property': [
        'error',
        {
          ignore: [
            // Three.js/React Three Fiber properties
            'args',
            'object',
            'attach',
            'geometry',
            'material',
            'position',
            'rotation',
            'scale',
            'intensity',
            'color',
            'distance',
            'decay',
            'angle',
            'penumbra',
            'castShadow',
            'receiveShadow',
            'map',
            'transparent',
            'opacity',
            'wireframe',
            'flatShading',
            'roughness',
            'metalness',
            'clearcoat',
            'clearcoatRoughness',
            'normalScale',
            'displacementScale',
            'aoMapIntensity',
            'lightMapIntensity',
            'emissiveIntensity',
            'envMapIntensity',
            'refractionRatio',
            'fog',
            'toneMapped',
            'userData',
            // React Three Drei properties
            'enablePan',
            'enableZoom',
            'enableRotate',
            'enableDamping',
            'dampingFactor',
            'autoRotate',
            'autoRotateSpeed',
            'target',
            'minDistance',
            'maxDistance',
            'minPolarAngle',
            'maxPolarAngle',
            'minAzimuthAngle',
            'maxAzimuthAngle',
          ],
        },
      ],
      'react/require-render-return': 'error',
      'react/self-closing-comp': 'error',
      'react/jsx-fragments': ['error', 'syntax'],
      'react/jsx-no-useless-fragment': 'error',
      'react/jsx-boolean-value': ['error', 'never'],
      'react/jsx-curly-brace-presence': [
        'error',
        { props: 'never', children: 'never' },
      ],

      // React Hooks
      ...reactHooks.configs.recommended.rules,

      // React Refresh (for development)
      'react-refresh/only-export-components': [
        'warn',
        { allowConstantExport: true },
      ],

      // Accessibility rules
      ...jsxA11y.configs.recommended.rules,
      'jsx-a11y/anchor-is-valid': ['error', { aspects: ['invalidHref'] }],

      // Import rules (simplified to avoid resolver conflicts)
      'import/order': [
        'error',
        {
          groups: [
            'builtin',
            'external',
            'internal',
            'parent',
            'sibling',
            'index',
          ],
          'newlines-between': 'always',
          alphabetize: {
            order: 'asc',
            caseInsensitive: true,
          },
        },
      ],
      'import/no-duplicates': 'error',

      // Unicorn rules (modern JavaScript practices)
      'unicorn/filename-case': [
        'error',
        {
          cases: {
            kebabCase: true,
            pascalCase: true,
          },
        },
      ],
      'unicorn/no-null': 'off', // We use null in React/DOM contexts
      'unicorn/prevent-abbreviations': 'off', // Too aggressive for our use case
      'unicorn/prefer-top-level-await': 'off', // Not always appropriate
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
          classPropertiesAllowed: false,
        },
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
      'prefer-destructuring': [
        'error',
        {
          array: false,
          object: true,
        },
        {
          enforceForRenamedProperties: false,
        },
      ],

      // Performance and game development specific
      'no-loop-func': 'error',
      'no-inner-declarations': 'error',
      'no-implied-eval': 'error',
      'no-new-func': 'error',
    },
  },

  // Prettier compatibility - must be last
  prettier,
];
