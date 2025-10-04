# Development Tools

This directory contains various development and build tools for the ManifestRustTS project.

## 📁 Directory Structure

### `ai-texture-generator/`
**AI-powered texture generation system using OpenAI DALL-E 3**

- **Purpose**: Generate high-quality, seamless game textures for all biome types
- **Cost**: ~$1.60 for complete texture set (40 textures)
- **Output**: Professional PBR texture maps (albedo, normal, roughness, metallic)
- **Integration**: Drop-in replacement for existing procedural textures

**Quick Start:**
```bash
# From project root
./generate-textures.sh setup    # Test setup
./generate-textures.sh test     # Generate one biome
./generate-textures.sh          # Generate all textures
```

**Files:**
- `generate_ai_textures.py` - Main generation script
- `test_setup.py` - Setup verification and testing
- `requirements.txt` - Python dependencies
- `.env.example` - Environment variable template
- `README.md` - Complete setup and usage guide

---

## 🚀 Quick Access Scripts

### From Project Root:

```bash
# AI Texture Generation
./generate-textures.sh          # Interactive texture generation
./generate-textures.sh test     # Quick test (Forest biome only)
./generate-textures.sh setup    # Verify API and dependencies
```

### Direct Access:

```bash
# Navigate to specific tool
cd tools/ai-texture-generator
python3 generate_ai_textures.py --help
```

## 🔧 Adding New Tools

When adding new development tools:

1. Create a descriptive subdirectory in `tools/`
2. Include a README.md explaining the tool's purpose
3. Add any wrapper scripts to the project root if needed
4. Update this main tools README.md
5. Add appropriate .gitignore entries for generated files

## 📋 Planned Tools

Future development tools may include:

- **Asset Pipeline**: Automated asset processing and optimization
- **Deployment Scripts**: Build and deployment automation
- **Performance Profilers**: Game performance analysis tools
- **Code Generators**: Boilerplate and scaffolding generators
