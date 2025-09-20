// @ts-check
import js from '@eslint/js'
import tsParser from '@typescript-eslint/parser'
import tsPlugin from '@typescript-eslint/eslint-plugin'
import * as tseslint from 'typescript-eslint'
import reactPlugin from 'eslint-plugin-react'
import reactHooks from 'eslint-plugin-react-hooks'
import globals from 'globals'

export default [
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
        plugins: { '@typescript-eslint': tsPlugin, react: reactPlugin, 'react-hooks': reactHooks },
        rules: {
            'react/react-in-jsx-scope': 'off',
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
