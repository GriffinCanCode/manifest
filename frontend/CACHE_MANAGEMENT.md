# Vite Cache Management

This project includes automatic cache management to prevent **504 "Outdated Optimize Dep"** errors that can occur with Vite's dependency optimization.

## Problem

Vite pre-bundles dependencies for better development performance, but sometimes these cached dependencies become stale, leading to:

- 504 "Outdated Optimize Dep" errors
- Failed resource loading
- Development server instability

## Solution

We've integrated automatic cache clearing into the development and build processes.

## Available Scripts

### Development

```bash
# Standard development (with automatic cache clearing)
npm run dev

# Safe development (skips cache clearing - use if needed)
npm run dev:safe

# Fresh development (cache clearing + reinstall dependencies)
npm run dev:fresh
```

### Building

```bash
# Standard build (with automatic cache clearing)
npm run build

# Safe build (skips cache clearing - use if needed)
npm run build:safe
```

### Manual Cache Management

```bash
# Clear caches manually
npm run clear-cache
```

## What Gets Cleared

The cache clearing script removes:

- `node_modules/.vite` - Vite's dependency optimization cache
- `node_modules/.cache` - General Node.js cache
- `.vite` - Local Vite cache
- `dist` - Build output directory
- npm cache (via `npm cache clean --force`)

## Configuration

The cache clearing behavior is configured in:

- `clear-cache.js` - Main cache clearing script
- `package.json` - Script definitions
- `vite.config.ts` - Enhanced dependency optimization settings

## Vite Configuration Enhancements

The `vite.config.ts` includes:

- Aggressive dependency optimization with `force: true`
- Comprehensive list of dependencies for pre-bundling
- Enhanced cache invalidation settings
- Entry point specification for better optimization

## When to Use Safe Mode

Use the `:safe` variants when:

- You're confident your cache is clean
- You want faster startup times
- You're working on non-dependency related changes
- You're debugging cache-related issues

## Troubleshooting

If you still encounter 504 errors:

1. Try `npm run dev:fresh` for a complete reset
2. Manually delete `node_modules` and run `npm install`
3. Check for any global npm cache issues
4. Verify your Node.js version is compatible

## Integration with Existing Scripts

The start scripts in `/scripts/start-frontend.sh` automatically use the cache-clearing development process, with helpful console output indicating when cache clearing occurs.
