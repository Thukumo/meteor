// @ts-check
import js from '@eslint/js'
import tsParser from '@typescript-eslint/parser'
import tsPlugin from '@typescript-eslint/eslint-plugin'
import * as tseslint from 'typescript-eslint'
import reactPlugin from 'eslint-plugin-react'
import reactHooks from 'eslint-plugin-react-hooks'
import globals from 'globals'
import jsxA11y from 'eslint-plugin-jsx-a11y'

export default [
    // Ignore build outputs
    { ignores: ['dist/**'] },
    js.configs.recommended,
    // TS recommended (型情報なし)
    ...tseslint.configs.recommended,
    // TS recommended (型情報あり)
    ...tseslint.configs.recommendedTypeChecked,
    {
        files: ['**/*.{ts,tsx}'],
        languageOptions: {
            parser: tsParser,
            parserOptions: {
                ecmaVersion: 'latest',
                sourceType: 'module',
                project: ['./tsconfig.json'],
                tsconfigRootDir: new URL('.', import.meta.url).pathname,
            },
            globals: { ...globals.browser, ...globals.es2021 },
        },
        plugins: {
            '@typescript-eslint': tsPlugin,
            react: reactPlugin,
            'react-hooks': reactHooks,
            'jsx-a11y': jsxA11y,
        },
        rules: {
            'react/react-in-jsx-scope': 'off',
            // Accessibility / React: enforce button type
            'react/button-has-type': 'error',
            'jsx-a11y/alt-text': 'warn',
            'jsx-a11y/click-events-have-key-events': 'warn',

            // React best practices
            'react/no-array-index-key': 'warn',
            'react/jsx-no-useless-fragment': ['warn', { allowExpressions: true }],

            // General JS/TS hygiene
            'prefer-const': 'warn',
            eqeqeq: ['warn', 'smart'],
            'no-console': ['warn', { allow: ['warn', 'error'] }],
            '@typescript-eslint/consistent-type-imports': 'warn',
            'no-undef': 'off',
            'no-unused-vars': 'off',
            '@typescript-eslint/no-unused-vars': [
                'warn',
                {
                    argsIgnorePattern: '^_',
                    varsIgnorePattern: '^_',
                    caughtErrorsIgnorePattern: '^_',
                },
            ],
            ...reactHooks.configs.recommended.rules,
        },
        settings: { react: { version: 'detect' } },
    },
]
