/**
 * HexBorderRenderer
 * Efficient border/edge rendering for hex tiles using instanced line rendering
 * Provides political borders, terrain edges, and selection highlights
 */

import { useFrame, useThree } from '@react-three/fiber';
import { useCallback, useEffect, useMemo, useRef } from 'react';
import { Color, Vector2, Vector3, type Group } from 'three';
import { Line2, LineGeometry, LineMaterial } from 'three-stdlib';

import { useRenderStore } from '../../../../stores/render-store';
import {
  HexUtils,
  TerrainType,
  type GameTile,
} from '../../../../utils/game-types';

interface HexBorderRendererProps {
  readonly tiles: readonly GameTile[];
  readonly selectedTileId?: number;
  readonly highlightedTiles?: ReadonlySet<number>;
  readonly showPoliticalBorders?: boolean;
  readonly showTerrainBorders?: boolean;
  readonly showSelectionBorder?: boolean;
  readonly borderWidth?: number;
  readonly maxRenderDistance?: number;
}

interface BorderLine {
  id: string;
  points: Vector3[];
  color: Color;
  width: number;
  visible: boolean;
  type: 'political' | 'terrain' | 'selection' | 'highlight';
}

/**
 * High-performance hex border renderer using Line2 from three-line2
 * Supports various border types with distance-based culling
 */
export const HexBorderRenderer: React.FC<HexBorderRendererProps> = ({
  tiles,
  selectedTileId,
  highlightedTiles = new Set(),
  showPoliticalBorders = true,
  showTerrainBorders = false,
  showSelectionBorder = true,
  borderWidth = 0.05,
  maxRenderDistance = 50,
}) => {
  const { camera } = useThree();
  const { quality } = useRenderStore();
  const groupRef = useRef<Group>(null);
  const borderLinesRef = useRef<Map<string, BorderLine>>(new Map());
  const line2RefsRef = useRef<Map<string, Line2>>(new Map());

  // Border colors configuration
  const borderColors = useMemo(
    () => ({
      political: new Color('#ff6b6b'),
      terrain: new Color('#4ecdc4'),
      selection: new Color('#ffe66d'),
      highlight: new Color('#a8e6cf'),
      water: new Color('#1e40af'),
      land: new Color('#22c55e'),
    }),
    []
  );

  // Line width based on quality settings
  const effectiveBorderWidth = useMemo(() => {
    const multiplier =
      quality.level === 'low' ? 0.5 : quality.level === 'medium' ? 1 : 1.5;
    return borderWidth * multiplier;
  }, [quality.level, borderWidth]);

  // Get hex edge points for a tile
  const getHexEdgePoints = useCallback((tile: GameTile): Vector3[] => {
    const [centerX, centerZ] = HexUtils.hexToPixel(tile.hex);
    const centerY = tile.elevation * 0.5;
    const size = 1.0; // Hex size
    const height = size * 0.866025404; // sqrt(3) / 2

    // Six vertices of a hexagon (flat-top)
    const vertices = [
      new Vector3(centerX + size, centerY, centerZ), // East
      new Vector3(centerX + size * 0.5, centerY, centerZ + height), // Southeast
      new Vector3(centerX - size * 0.5, centerY, centerZ + height), // Southwest
      new Vector3(centerX - size, centerY, centerZ), // West
      new Vector3(centerX - size * 0.5, centerY, centerZ - height), // Northwest
      new Vector3(centerX + size * 0.5, centerY, centerZ - height), // Northeast
    ];

    return vertices;
  }, []);

  // Generate political border lines
  // TODO: Integrate with backend ownership system once GameTile interface includes owner property
  const generatePoliticalBorders = useCallback((): BorderLine[] => {
    // Political borders disabled until ownership system is integrated
    // Backend has TileOwnershipClaims but frontend GameTile interface doesn't include owner
    return [];
  }, []);

  // Generate terrain border lines
  const generateTerrainBorders = useCallback((): BorderLine[] => {
    const borders: BorderLine[] = [];
    const tileMap = new Map(tiles.map(t => [t.id, t]));

    for (const tile of tiles) {
      const edgePoints = getHexEdgePoints(tile);

      // Check each edge against neighbors
      for (let i = 0; i < 6; i++) {
        // Create neighbor coordinates manually
        const neighborOffsets = [
          { q: 1, r: 0 }, // East
          { q: 1, r: -1 }, // Northeast
          { q: 0, r: -1 }, // Northwest
          { q: -1, r: 0 }, // West
          { q: -1, r: 1 }, // Southwest
          { q: 0, r: 1 }, // Southeast
        ];

        const offset = neighborOffsets[i];
        const neighbor = { q: tile.hex.q + offset.q, r: tile.hex.r + offset.r };
        const neighborTile = Array.from(tileMap.values()).find(
          t => t.hex.q === neighbor.q && t.hex.r === neighbor.r
        );

        // Draw border if neighbor has different terrain
        if (neighborTile && neighborTile.terrain !== tile.terrain) {
          const start = edgePoints[i];
          const end = edgePoints[(i + 1) % 6];

          // Choose color based on terrain types
          let color = borderColors.terrain;
          if (
            tile.terrain === TerrainType.Ocean ||
            neighborTile.terrain === TerrainType.Ocean
          ) {
            color = borderColors.water;
          }

          borders.push({
            id: `terrain-${tile.id}-${i}`,
            points: [start, end],
            color,
            width: effectiveBorderWidth,
            visible: true,
            type: 'terrain',
          });
        }
      }
    }

    return borders;
  }, [
    tiles,
    getHexEdgePoints,
    borderColors.terrain,
    borderColors.water,
    effectiveBorderWidth,
  ]);

  // Generate selection border
  const generateSelectionBorders = useCallback((): BorderLine[] => {
    const borders: BorderLine[] = [];

    const selectedTile = tiles.find(t => t.id === selectedTileId);
    if (selectedTile) {
      const edgePoints = getHexEdgePoints(selectedTile);

      // Create full hex border
      const allPoints = [...edgePoints, edgePoints[0]]; // Close the loop

      borders.push({
        id: `selection-${selectedTileId}`,
        points: allPoints,
        color: borderColors.selection,
        width: effectiveBorderWidth * 2,
        visible: true,
        type: 'selection',
      });
    }

    return borders;
  }, [
    tiles,
    selectedTileId,
    getHexEdgePoints,
    borderColors.selection,
    effectiveBorderWidth,
  ]);

  // Generate highlight borders
  const generateHighlightBorders = useCallback((): BorderLine[] => {
    const borders: BorderLine[] = [];

    for (const tileId of highlightedTiles) {
      const tile = tiles.find(t => t.id === tileId);
      if (!tile) continue;

      const edgePoints = getHexEdgePoints(tile);
      const allPoints = [...edgePoints, edgePoints[0]]; // Close the loop

      borders.push({
        id: `highlight-${tileId}`,
        points: allPoints,
        color: borderColors.highlight,
        width: effectiveBorderWidth * 1.5,
        visible: true,
        type: 'highlight',
      });
    }

    return borders;
  }, [
    tiles,
    highlightedTiles,
    getHexEdgePoints,
    borderColors.highlight,
    effectiveBorderWidth,
  ]);

  // Create Line2 from border line data
  const createLine2 = useCallback((borderLine: BorderLine): Line2 => {
    const geometry = new LineGeometry();
    const positions: number[] = [];

    for (const point of borderLine.points) {
      positions.push(point.x, point.y, point.z);
    }

    geometry.setPositions(positions);

    const material = new LineMaterial({
      color: borderLine.color.getHex(),
      linewidth: borderLine.width,
      resolution: new Vector2(window.innerWidth, window.innerHeight),
      alphaToCoverage: true,
    });

    const line = new Line2(geometry, material);
    line.computeLineDistances();
    line.visible = borderLine.visible;

    return line;
  }, []);

  // Update all border lines
  const updateBorderLines = useCallback(() => {
    if (!camera) return;

    const allBorders: BorderLine[] = [];

    if (showPoliticalBorders) {
      allBorders.push(...generatePoliticalBorders());
    }

    if (showTerrainBorders) {
      allBorders.push(...generateTerrainBorders());
    }

    if (showSelectionBorder) {
      allBorders.push(...generateSelectionBorders());
    }

    allBorders.push(...generateHighlightBorders());

    // Distance culling
    const cameraPosition = camera.position;
    for (const border of allBorders) {
      if (border.points.length > 0) {
        const distance = cameraPosition.distanceTo(border.points[0]);
        border.visible = distance < maxRenderDistance;
      }
    }

    // Update border lines map
    const newBorderLines = new Map<string, BorderLine>();
    for (const border of allBorders) {
      newBorderLines.set(border.id, border);
    }

    borderLinesRef.current = newBorderLines;
  }, [
    camera,
    showPoliticalBorders,
    showTerrainBorders,
    showSelectionBorder,
    generatePoliticalBorders,
    generateTerrainBorders,
    generateSelectionBorders,
    generateHighlightBorders,
    maxRenderDistance,
  ]);

  // Update Line2 objects
  const updateLine2Objects = useCallback(() => {
    if (!groupRef.current) return;

    const group = groupRef.current;
    const borderLines = borderLinesRef.current;
    const line2Refs = line2RefsRef.current;

    // Remove old lines
    for (const [id, line2] of line2Refs) {
      if (!borderLines.has(id)) {
        group.remove(line2);
        line2.geometry.dispose();
        line2.material.dispose();
        line2Refs.delete(id);
      }
    }

    // Add or update lines
    for (const [id, borderLine] of borderLines) {
      if (borderLine.points.length < 2) continue;

      let line2 = line2Refs.get(id);

      if (!line2) {
        line2 = createLine2(borderLine);
        line2Refs.set(id, line2);
        group.add(line2);
      } else {
        // Update existing line
        const { geometry } = line2;
        const positions: number[] = [];

        for (const point of borderLine.points) {
          positions.push(point.x, point.y, point.z);
        }

        geometry.setPositions(positions);
        line2.computeLineDistances();

        const { material } = line2;
        material.color.setHex(borderLine.color.getHex());
        material.linewidth = borderLine.width;
        material.needsUpdate = true;
      }

      line2.visible = borderLine.visible;
    }
  }, [createLine2]);

  // Update on frame
  useFrame(() => {
    updateBorderLines();
    updateLine2Objects();
  });

  // Initialize
  useEffect(() => {
    updateBorderLines();
    updateLine2Objects();
  }, [updateBorderLines, updateLine2Objects]);

  return <group ref={groupRef} name='hex-border-renderer' />;
};
