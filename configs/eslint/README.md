# ESLint Configuration for ManifestRustTS

This directory contains the ESLint configuration setup for the ManifestRustTS project, following 2025 best practices.

## Features

### Modern ESLint Flat Config (ESLint 9.x)
- Uses the new flat configuration format
- TypeScript-first approach with full type checking
- React 18+ optimized rules
- Accessibility (a11y) enforcement
- Import organization and validation
- Modern JavaScript/TypeScript practices

### Key Plugins Integrated

1. **TypeScript ESLint v8**: Latest TypeScript linting with type-aware rules
2. **React Plugin**: React-specific rules and best practices
3. **React Hooks**: Hooks-specific linting
4. **JSX A11y**: Accessibility linting for React components
5. **Import Plugin**: Import statement organization and validation
6. **Unicorn**: Modern JavaScript practices enforcement
7. **Prefer Arrow**: Consistent arrow function usage

### Configuration Files

- `base.js` - Shared configuration that can be extended across the project
- `frontend/eslint.config.js` - Main frontend configuration with React-specific rules
- `frontend/.eslintignore` - Files and directories to ignore during linting
- `frontend/.prettierrc` - Prettier configuration for code formatting
- `frontend/.prettierignore` - Files to ignore during formatting

### Scripts Available

From the `frontend/` directory:

```bash
# Lint all files
npm run lint

# Lint and fix issues automatically
npm run lint:fix

# Format code with Prettier
npm run format

# Check formatting without fixing
npm run format:check

# Run all checks (type check + lint + format check)
npm run check

# Type check only
npm run type-check
```

### VS Code Integration

The `.vscode/settings.json` file is configured to:
- Enable ESLint flat config support
- Auto-fix on save
- Use Prettier as the default formatter
- Organize imports on save
- Work seamlessly with the linting setup

### Key Rules Highlights

#### TypeScript
- Consistent type imports with separate type imports style
- Prefer nullish coalescing and optional chaining
- Strict unused variable checking with underscore prefix exemption
- Type-aware rules for async/await and promises

#### React
- React 17+ optimized (no need for React import)
- Strict JSX key requirements
- Fragment syntax preference
- Component naming conventions
- Accessibility enforcement

#### Import Organization
- Alphabetical sorting within groups
- Logical grouping (builtin, external, internal, relative)
- Path mapping support for `@/` aliases
- Cycle detection and duplicate prevention

#### Modern JavaScript
- Arrow function preference
- Modern array methods preference
- Template literal preference
- ES6+ feature usage

### Performance Optimizations

The configuration is optimized for large codebases:
- Efficient file matching patterns
- Type-aware rules only where necessary
- Proper ignore patterns to skip unnecessary files
- Parallel processing support

### Accessibility

JSX A11y rules ensure:
- Proper ARIA usage
- Keyboard navigation support
- Screen reader compatibility
- Color contrast considerations
- Semantic HTML structure

This setup provides a comprehensive, modern, and performant linting solution for the ManifestRustTS project.
