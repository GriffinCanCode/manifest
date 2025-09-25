import { Environment, Html, OrbitControls } from '@react-three/drei';
import { Canvas, useFrame } from '@react-three/fiber';
import React, { Suspense, useCallback, useMemo, useRef, useState } from 'react';
import * as THREE from 'three';

// Terrain type definitions matching backend
enum TerrainType {
  Ocean = 'ocean',
  Grassland = 'grassland',
  Plains = 'plains',
  Desert = 'desert',
  Tundra = 'tundra',
  Snow = 'snow',
  Forest = 'forest',
  Jungle = 'jungle',
  Hills = 'hills',
  Mountain = 'mountain',
}

// Hex coordinate system
interface HexCoord {
  q: number;
  r: number;
}

// Tile data structure
interface GameTile {
  id: number;
  hex: HexCoord;
  terrain: TerrainType;
  elevation: number;
  resources?: string[];
  improvements?: string[];
  units?: GameUnit[];
}

// Game unit structure
interface GameUnit {
  id: number;
  type: string;
  playerId: number;
  health: number;
  position: HexCoord;
  isSelected?: boolean;
}

// Hex geometry and positioning utilities
class HexUtils {
  static readonly HEX_SIZE = 1.0;
  static readonly HEX_HEIGHT = Math.sqrt(3) * HexUtils.HEX_SIZE;
  static readonly HEX_WIDTH = 2 * HexUtils.HEX_SIZE;

  static hexToPixel(hex: HexCoord): [number, number] {
    const x = HexUtils.HEX_SIZE * ((3 / 2) * hex.q);
    const z =
      HexUtils.HEX_SIZE * ((Math.sqrt(3) / 2) * hex.q + Math.sqrt(3) * hex.r);
    return [x, z];
  }

  static pixelToHex(x: number, z: number): HexCoord {
    const q = ((2 / 3) * x) / HexUtils.HEX_SIZE;
    const r = ((-1 / 3) * x + (Math.sqrt(3) / 3) * z) / HexUtils.HEX_SIZE;
    return { q: Math.round(q), r: Math.round(r) };
  }

  static getNeighbors(hex: HexCoord): HexCoord[] {
    const directions = [
      { q: 1, r: 0 },
      { q: 1, r: -1 },
      { q: 0, r: -1 },
      { q: -1, r: 0 },
      { q: -1, r: 1 },
      { q: 0, r: 1 },
    ];
    return directions.map(dir => ({ q: hex.q + dir.q, r: hex.r + dir.r }));
  }
}

// Individual hex tile component
interface HexTileProps {
  tile: GameTile;
  onTileClick: (tile: GameTile) => void;
  isSelected?: boolean;
  isHighlighted?: boolean;
}

const HexTile: React.FC<HexTileProps> = ({
  tile,
  onTileClick,
  isSelected,
  isHighlighted,
}) => {
  const meshRef = useRef<THREE.Mesh>(null);
  const [hovered, setHovered] = useState(false);

  const [x, z] = HexUtils.hexToPixel(tile.hex);
  const y = tile.elevation * 0.1; // Scale elevation for visual effect

  // Generate hex geometry
  const hexGeometry = useMemo(() => {
    const geometry = new THREE.CylinderGeometry(
      HexUtils.HEX_SIZE,
      HexUtils.HEX_SIZE,
      0.1,
      6
    );
    return geometry;
  }, []);

  // Terrain-based colors and materials
  const terrainMaterial = useMemo(() => {
    const colors = {
      [TerrainType.Ocean]: '#2b5797',
      [TerrainType.Grassland]: '#4d7c0f',
      [TerrainType.Plains]: '#84cc16',
      [TerrainType.Desert]: '#eab308',
      [TerrainType.Tundra]: '#94a3b8',
      [TerrainType.Snow]: '#f8fafc',
      [TerrainType.Forest]: '#166534',
      [TerrainType.Jungle]: '#14532d',
      [TerrainType.Hills]: '#a3a3a3',
      [TerrainType.Mountain]: '#525252',
    };

    const baseColor = colors[tile.terrain] || '#64748b';
    const color = hovered
      ? '#ffffff'
      : isSelected
        ? '#fbbf24'
        : isHighlighted
          ? '#34d399'
          : baseColor;

    return new THREE.MeshStandardMaterial({
      color,
      roughness: tile.terrain === TerrainType.Ocean ? 0.1 : 0.7,
      metalness: tile.terrain === TerrainType.Mountain ? 0.3 : 0.1,
    });
  }, [tile.terrain, hovered, isSelected, isHighlighted]);

  const handleClick = useCallback(
    (event: THREE.Event) => {
      event.stopPropagation();
      onTileClick(tile);
    },
    [tile, onTileClick]
  );

  return (
    <group position={[x, y, z]}>
      {/* Main hex tile */}
      <mesh
        ref={meshRef}
        geometry={hexGeometry}
        material={terrainMaterial}
        onClick={handleClick}
        onPointerOver={() => setHovered(true)}
        onPointerOut={() => setHovered(false)}
      />

      {/* Elevation indicator for mountains/hills */}
      {(tile.terrain === TerrainType.Mountain ||
        tile.terrain === TerrainType.Hills) && (
        <mesh position={[0, 0.1, 0]}>
          <coneGeometry args={[0.3, 0.4, 4]} />
          <meshStandardMaterial
            color={
              tile.terrain === TerrainType.Mountain ? '#404040' : '#737373'
            }
          />
        </mesh>
      )}

      {/* Forest/Jungle vegetation */}
      {(tile.terrain === TerrainType.Forest ||
        tile.terrain === TerrainType.Jungle) && (
        <>
          <mesh position={[0.2, 0.15, 0.1]}>
            <coneGeometry args={[0.1, 0.3, 4]} />
            <meshStandardMaterial
              color={
                tile.terrain === TerrainType.Forest ? '#166534' : '#14532d'
              }
            />
          </mesh>
          <mesh position={[-0.1, 0.15, -0.2]}>
            <coneGeometry args={[0.08, 0.25, 4]} />
            <meshStandardMaterial
              color={
                tile.terrain === TerrainType.Forest ? '#166534' : '#14532d'
              }
            />
          </mesh>
        </>
      )}

      {/* Resource indicators */}
      {tile.resources &&
        tile.resources.map((resource, index) => (
          <mesh
            key={resource}
            position={[
              0.3 * Math.cos((index * Math.PI) / 3),
              0.05,
              0.3 * Math.sin((index * Math.PI) / 3),
            ]}
          >
            <sphereGeometry args={[0.05]} />
            <meshStandardMaterial
              color={
                resource === 'gold'
                  ? '#fbbf24'
                  : resource === 'iron'
                    ? '#6b7280'
                    : '#ef4444'
              }
            />
          </mesh>
        ))}

      {/* Coordinate display on hover */}
      {hovered && (
        <Html position={[0, 0.3, 0]} center>
          <div className='hex-tooltip'>
            <div>
              Hex: {tile.hex.q}, {tile.hex.r}
            </div>
            <div>Terrain: {tile.terrain}</div>
            <div>Elevation: {tile.elevation.toFixed(1)}</div>
            {tile.resources && (
              <div>Resources: {tile.resources.join(', ')}</div>
            )}
          </div>
        </Html>
      )}
    </group>
  );
};

// Game unit component
interface GameUnitComponentProps {
  unit: GameUnit;
  onUnitClick: (unit: GameUnit) => void;
}

const GameUnitComponent: React.FC<GameUnitComponentProps> = ({
  unit,
  onUnitClick,
}) => {
  const meshRef = useRef<THREE.Mesh>(null);
  const [hovered, setHovered] = useState(false);

  const [x, z] = HexUtils.hexToPixel(unit.position);
  const y = 0.2; // Units float above terrain

  // Unit colors based on player
  const playerColors = ['#ef4444', '#3b82f6', '#10b981', '#f59e0b', '#8b5cf6'];
  const unitColor = playerColors[unit.playerId % playerColors.length];

  // Animate unit (bob up and down)
  useFrame(({ clock }) => {
    if (meshRef.current) {
      meshRef.current.position.y =
        y + Math.sin(clock.getElapsedTime() * 2) * 0.05;
    }
  });

  const handleClick = useCallback(
    (event: THREE.Event) => {
      event.stopPropagation();
      onUnitClick(unit);
    },
    [unit, onUnitClick]
  );

  return (
    <group position={[x, y, z]}>
      {/* Unit body */}
      <mesh
        ref={meshRef}
        onClick={handleClick}
        onPointerOver={() => setHovered(true)}
        onPointerOut={() => setHovered(false)}
      >
        <cylinderGeometry args={[0.15, 0.15, 0.3]} />
        <meshStandardMaterial
          color={hovered || unit.isSelected ? '#ffffff' : unitColor}
          emissive={unit.isSelected ? '#444444' : '#000000'}
        />
      </mesh>

      {/* Unit type indicator */}
      <mesh position={[0, 0.2, 0]}>
        <sphereGeometry args={[0.08]} />
        <meshStandardMaterial color={unitColor} />
      </mesh>

      {/* Health bar */}
      <Html position={[0, 0.4, 0]} center>
        <div className='unit-health-bar'>
          <div
            className='health-fill'
            style={{
              width: `${(unit.health / 100) * 30}px`,
              height: '4px',
              backgroundColor:
                unit.health > 50
                  ? '#10b981'
                  : unit.health > 25
                    ? '#f59e0b'
                    : '#ef4444',
              border: '1px solid #000',
            }}
          />
        </div>
      </Html>

      {/* Selection indicator */}
      {unit.isSelected && (
        <mesh position={[0, -0.05, 0]} rotation={[-Math.PI / 2, 0, 0]}>
          <ringGeometry args={[0.25, 0.3, 8]} />
          <meshBasicMaterial color='#fbbf24' transparent opacity={0.7} />
        </mesh>
      )}
    </group>
  );
};

// Main game scene component
const GameScene: React.FC = () => {
  const [selectedTile, setSelectedTile] = useState<GameTile | null>(null);
  const [selectedUnit, setSelectedUnit] = useState<GameUnit | null>(null);
  const [highlightedTiles, setHighlightedTiles] = useState<Set<number>>(
    new Set()
  );

  // Generate sample game world
  const gameWorld = useMemo(() => {
    const tiles: GameTile[] = [];
    const units: GameUnit[] = [];
    let tileId = 0;
    let unitId = 0;

    // Generate hex grid
    const mapRadius = 8;
    for (let q = -mapRadius; q <= mapRadius; q++) {
      const r1 = Math.max(-mapRadius, -q - mapRadius);
      const r2 = Math.min(mapRadius, -q + mapRadius);
      for (let r = r1; r <= r2; r++) {
        const hex = { q, r };

        // Determine terrain based on position (simple algorithm)
        let terrain = TerrainType.Grassland;
        const distance = Math.abs(q) + Math.abs(r) + Math.abs(-q - r);
        const noise =
          Math.sin(q * 0.3) * Math.cos(r * 0.4) * Math.sin((q + r) * 0.2);

        if (distance < 2) terrain = TerrainType.Plains;
        else if (distance > 6) terrain = TerrainType.Ocean;
        else if (noise > 0.3) terrain = TerrainType.Forest;
        else if (noise < -0.3) terrain = TerrainType.Hills;
        else if (Math.random() > 0.8) terrain = TerrainType.Mountain;
        else if (Math.random() > 0.9) terrain = TerrainType.Desert;

        const elevation = Math.max(
          0,
          noise * 2 +
            (terrain === TerrainType.Mountain
              ? 3
              : terrain === TerrainType.Hills
                ? 1.5
                : terrain === TerrainType.Ocean
                  ? -0.5
                  : 0)
        );

        // Add resources randomly
        const resources: string[] = [];
        if (Math.random() > 0.85) {
          if (terrain === TerrainType.Mountain) resources.push('iron');
          else if (terrain === TerrainType.Hills) resources.push('stone');
          else if (terrain === TerrainType.Desert) resources.push('gold');
          else resources.push('food');
        }

        tiles.push({
          id: tileId++,
          hex,
          terrain,
          elevation,
          resources: resources.length > 0 ? resources : undefined,
        });

        // Add units randomly
        if (
          Math.random() > 0.95 &&
          terrain !== TerrainType.Ocean &&
          distance < 6
        ) {
          units.push({
            id: unitId++,
            type: 'warrior',
            playerId: Math.floor(Math.random() * 4),
            health: Math.floor(Math.random() * 40) + 60,
            position: hex,
          });
        }
      }
    }

    return { tiles, units };
  }, []);

  const handleTileClick = useCallback(
    (tile: GameTile) => {
      setSelectedTile(tile);
      setSelectedUnit(null);

      // Highlight neighboring tiles
      const neighbors = HexUtils.getNeighbors(tile.hex);
      const neighborIds = gameWorld.tiles
        .filter(t => neighbors.some(n => n.q === t.hex.q && n.r === t.hex.r))
        .map(t => t.id);
      setHighlightedTiles(new Set(neighborIds));

      console.log('Selected tile:', tile);
    },
    [gameWorld.tiles]
  );

  const handleUnitClick = useCallback(
    (unit: GameUnit) => {
      setSelectedUnit(unit);
      setSelectedTile(null);

      // Highlight tiles in movement range
      const movementRange = 3;
      const inRange = gameWorld.tiles.filter(tile => {
        const distance =
          Math.abs(tile.hex.q - unit.position.q) +
          Math.abs(tile.hex.r - unit.position.r) +
          Math.abs(
            -tile.hex.q - tile.hex.r + unit.position.q + unit.position.r
          );
        return distance <= movementRange;
      });
      setHighlightedTiles(new Set(inRange.map(t => t.id)));

      console.log('Selected unit:', unit);
    },
    [gameWorld.tiles]
  );

  return (
    <>
      {/* Lighting setup */}
      <ambientLight intensity={0.3} />
      <directionalLight
        position={[10, 20, 10]}
        intensity={1}
        castShadow
        shadow-camera-left={-50}
        shadow-camera-right={50}
        shadow-camera-top={50}
        shadow-camera-bottom={-50}
      />
      <pointLight position={[0, 10, 0]} intensity={0.3} />

      {/* Render all hex tiles */}
      {gameWorld.tiles.map(tile => (
        <HexTile
          key={tile.id}
          tile={tile}
          onTileClick={handleTileClick}
          isSelected={selectedTile?.id === tile.id}
          isHighlighted={highlightedTiles.has(tile.id)}
        />
      ))}

      {/* Render all units */}
      {gameWorld.units.map(unit => (
        <GameUnitComponent
          key={unit.id}
          unit={{ ...unit, isSelected: selectedUnit?.id === unit.id }}
          onUnitClick={handleUnitClick}
        />
      ))}

      {/* Game UI overlays */}
      {selectedTile && (
        <Html position={[5, 5, 5]} transform={false}>
          <div className='game-info-panel'>
            <h3>Tile Info</h3>
            <p>
              Position: ({selectedTile.hex.q}, {selectedTile.hex.r})
            </p>
            <p>Terrain: {selectedTile.terrain}</p>
            <p>Elevation: {selectedTile.elevation.toFixed(1)}</p>
            {selectedTile.resources && (
              <p>Resources: {selectedTile.resources.join(', ')}</p>
            )}
          </div>
        </Html>
      )}

      {selectedUnit && (
        <Html position={[5, 3, 5]} transform={false}>
          <div className='game-info-panel'>
            <h3>Unit Info</h3>
            <p>Type: {selectedUnit.type}</p>
            <p>Player: {selectedUnit.playerId + 1}</p>
            <p>Health: {selectedUnit.health}%</p>
            <p>
              Position: ({selectedUnit.position.q}, {selectedUnit.position.r})
            </p>
          </div>
        </Html>
      )}

      {/* Camera controls */}
      <OrbitControls
        enablePan
        enableZoom
        enableRotate
        minDistance={8}
        maxDistance={50}
        minPolarAngle={Math.PI / 6}
        maxPolarAngle={Math.PI / 2.2}
        maxAzimuthAngle={Math.PI / 4}
        minAzimuthAngle={-Math.PI / 4}
      />

      {/* Environment */}
      <Environment preset='dawn' />

      {/* Fog for depth */}
      <fog attach='fog' args={['#87CEEB', 20, 80]} />
    </>
  );
};

const GameCanvas: React.FC = () => {
  return (
    <div className='game-canvas'>
      <Canvas
        camera={{
          position: [15, 15, 15],
          fov: 65,
          near: 0.1,
          far: 1000,
        }}
        shadows
        dpr={[1, 2]}
        gl={{
          antialias: true,
          alpha: false,
          powerPreference: 'high-performance',
        }}
        performance={{
          min: 0.1,
        }}
      >
        <Suspense
          fallback={
            <Html center>
              <div
                style={{
                  color: 'white',
                  fontSize: '18px',
                  fontWeight: 'bold',
                  textAlign: 'center',
                  background: 'rgba(0,0,0,0.7)',
                  padding: '20px',
                  borderRadius: '10px',
                }}
              >
                Loading World...
              </div>
            </Html>
          }
        >
          <GameScene />
        </Suspense>
      </Canvas>

      {/* Game Controls UI */}
      <div className='game-controls'>
        <div className='control-panel'>
          <h3>Game Controls</h3>
          <p>Click tiles and units to select them</p>
          <p>Mouse: Rotate camera</p>
          <p>Scroll: Zoom in/out</p>
          <p>Right-click + drag: Pan</p>
        </div>
      </div>

      <style>{`
        .game-canvas {
          width: 100%;
          height: 100%;
          position: relative;
          background: linear-gradient(to bottom, #87CEEB 0%, #98D8E8 50%, #B0E0E6 100%);
        }

        .hex-tooltip {
          background: rgba(0, 0, 0, 0.8);
          color: white;
          padding: 8px 12px;
          border-radius: 6px;
          font-size: 12px;
          line-height: 1.4;
          pointer-events: none;
          white-space: nowrap;
        }

        .game-info-panel {
          position: absolute;
          top: 20px;
          right: 20px;
          background: rgba(0, 0, 0, 0.85);
          color: white;
          padding: 16px;
          border-radius: 8px;
          font-size: 14px;
          min-width: 200px;
          backdrop-filter: blur(4px);
          border: 1px solid rgba(255, 255, 255, 0.2);
        }

        .game-info-panel h3 {
          margin: 0 0 12px 0;
          font-size: 16px;
          font-weight: bold;
          color: #fbbf24;
        }

        .game-info-panel p {
          margin: 6px 0;
          line-height: 1.4;
        }

        .unit-health-bar {
          pointer-events: none;
        }

        .health-fill {
          border-radius: 2px;
        }

        .control-panel {
          position: absolute;
          top: 20px;
          left: 20px;
          background: rgba(0, 0, 0, 0.75);
          color: white;
          padding: 16px;
          border-radius: 8px;
          font-size: 13px;
          max-width: 250px;
          backdrop-filter: blur(4px);
          border: 1px solid rgba(255, 255, 255, 0.15);
        }

        .control-panel h3 {
          margin: 0 0 10px 0;
          font-size: 15px;
          color: #34d399;
        }

        .control-panel p {
          margin: 4px 0;
          opacity: 0.9;
        }

        .game-controls {
          position: absolute;
          top: 0;
          left: 0;
          right: 0;
          bottom: 0;
          pointer-events: none;
        }

        .game-controls > * {
          pointer-events: auto;
        }

        /* Responsive design */
        @media (max-width: 768px) {
          .game-info-panel,
          .control-panel {
            font-size: 12px;
            padding: 12px;
            min-width: 150px;
            max-width: 180px;
          }
          
          .game-info-panel h3,
          .control-panel h3 {
            font-size: 14px;
          }
        }

        /* Performance optimizations */
        .game-canvas canvas {
          display: block;
          touch-action: manipulation;
        }
      `}</style>
    </div>
  );
};

export default GameCanvas;
