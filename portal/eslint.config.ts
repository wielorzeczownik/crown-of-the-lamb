import html from '@html-eslint/eslint-plugin';
import eslintConfigPrettier from 'eslint-config-prettier';
import simpleImportSort from 'eslint-plugin-simple-import-sort';
import sonarjs from 'eslint-plugin-sonarjs';
import unicorn from 'eslint-plugin-unicorn';
import tseslint from 'typescript-eslint';

export default [
  {
    ignores: ['node_modules/', 'dist/'],
  },
  ...tseslint.configs.recommendedTypeChecked,
  eslintConfigPrettier,
  unicorn.configs.recommended,
  sonarjs.configs?.recommended,
  {
    files: ['**/*.ts', '**/*.tsx', '**/*.js', '**/*.mjs'],
    languageOptions: {
      parserOptions: {
        projectService: true,
        tsconfigRootDir: import.meta.dirname,
      },
    },
    plugins: {
      'simple-import-sort': simpleImportSort,
    },
    rules: {
      'simple-import-sort/imports': 'error',
      'simple-import-sort/exports': 'error',
      '@typescript-eslint/no-unused-vars': [
        'error',
        { argsIgnorePattern: '^_', varsIgnorePattern: '^_' },
      ],
      '@typescript-eslint/consistent-type-imports': [
        'error',
        { disallowTypeAnnotations: false },
      ],
      '@typescript-eslint/no-explicit-any': 'warn',
      '@typescript-eslint/no-shadow': 'error',
      '@typescript-eslint/prefer-nullish-coalescing': [
        'error',
        { ignorePrimitives: { boolean: true } },
      ],
      '@typescript-eslint/prefer-optional-chain': 'error',
      eqeqeq: 'error',
      'no-void': ['error', { allowAsStatement: true }],
      'no-console': 'warn',
      'no-var': 'error',
      'prefer-const': 'error',
      'id-length': ['error', { min: 3, exceptions: ['ws', 'id', 'en', 'pl'] }],
      'sonarjs/no-unused-vars': 'off',
      'sonarjs/void-use': 'off',
      // Math.random() is used only for animation timing, not security
      'sonarjs/pseudo-random': 'off',
      'unicorn/prefer-at': 'off',
      'unicorn/prefer-query-selector': 'off',
      'unicorn/no-array-sort': 'off',
      // Conflicts with Prettier
      'unicorn/number-literal-case': 'off',
      'unicorn/name-replacements': [
        'error',
        {
          replacements: {
            mod: { moderator: true },
          },
        },
      ],
    },
  },
  {
    files: ['**/*.html'],
    ...tseslint.configs.disableTypeChecked,
  },
  {
    ...html.configs['flat/recommended'],
    files: ['**/*.html'],
    rules: {
      ...html.configs['flat/recommended'].rules,
      // Prettier owns HTML formatting
      '@html-eslint/indent': 'off',
      '@html-eslint/no-extra-spacing-tags': 'off',
      '@html-eslint/require-closing-tags': 'off',
      '@html-eslint/quotes': 'off',
      '@html-eslint/element-newline': 'off',
      '@html-eslint/attrs-newline': 'off',
      '@html-eslint/no-multiple-empty-lines': 'off',
      '@html-eslint/no-trailing-spaces': 'off',
      '@html-eslint/use-baseline': 'off',
      // The attribute sorting you asked for
      '@html-eslint/sort-attrs': 'warn',
    },
  },
];
