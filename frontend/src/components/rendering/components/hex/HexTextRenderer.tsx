/**
 * HexTextRenderer
 * High-performance text rendering for hex tiles using troika-three-text
 * Provides scalable, GPU-optimized text labels for coordinates, resources, etc.
 */

import { Text } from '@react-three/drei';
import { useFrame, useThree } from '@react-three/fiber';
import { useCallback, useEffect, useMemo, useRef } from 'react';
import { Vector3, type Group } from 'three';

import { useRenderStore } from '../../../../stores/render-store';
import {
  HexUtils,
  TerrainType,
  type GameTile,
} from '../../../../utils/game-types';

interface HexTextRendererProps {
  readonly tiles: readonly GameTile[];
  readonly showCoordinates?: boolean;
  readonly showResources?: boolean;
  readonly maxTextDistance?: number;
  readonly fontSize?: number;
}

interface TextElement {
  id: string;
  position: Vector3;
  text: string;
  color: string;
  scale: number;
  visible: boolean;
}

/**
 * Optimized text renderer for hex tiles
 * Uses distance-based culling and LOD for performance
 */
export const HexTextRenderer: React.FC<HexTextRendererProps> = ({
  tiles,
  showCoordinates = false,
  showResources = false,
  maxTextDistance = 20,
  fontSize = 0.5,
}) => {
  const { camera } = useThree();
  const { quality, debug } = useRenderStore();
  const groupRef = useRef<Group>(null);
  const textElementsRef = useRef<Map<number, TextElement>>(new Map());
  const lastCameraPositionRef = useRef<Vector3>(new Vector3());

  // Text configuration based on quality
  const textConfig = useMemo(() => {
    const baseConfig = {
      fontSize,
      maxWidth: 200,
      lineHeight: 1.2,
      letterSpacing: 0.02,
      textAlign: 'center' as const,
      anchorX: 'center' as const,
      anchorY: 'middle' as const,
    };

    switch (quality.level) {
      case 'low':
        return {
          ...baseConfig,
          fontSize: fontSize * 0.8,
          resolution: 64,
          renderOrder: 999,
        };
      case 'medium':
        return {
          ...baseConfig,
          resolution: 128,
          renderOrder: 999,
        };
      case 'high':
        return {
          ...baseConfig,
          resolution: 256,
          renderOrder: 999,
        };
      default:
        return baseConfig;
    }
  }, [quality.level, fontSize]);

  // Get resource symbol for display
  const getResourceSymbol = useCallback((resourceType: string): string => {
    const symbols: Record<string, string> = {
      iron: '⛏',
      gold: '🥇',
      oil: '🛢',
      coal: '⚫',
      stone: '🗿',
      wood: '🌲',
      food: '🌾',
      horses: '🐎',
      fish: '🐟',
      gems: '💎',
    };
    return symbols[resourceType] || '●';
  }, []);

  // Generate text content for tile
  const generateTextContent = useCallback(
    (tile: GameTile): string => {
      const parts: string[] = [];

      if (showCoordinates) {
        parts.push(`(${tile.hex.q}, ${tile.hex.r})`);
      }

      if (showResources && tile.resources && tile.resources.length > 0) {
        const resourceSymbols = tile.resources
          .slice(0, 3) // Show max 3 resources
          .map(resource => getResourceSymbol(resource))
          .join(' ');
        parts.push(resourceSymbols);
      }

      return parts.join('\n');
    },
    [showCoordinates, showResources, getResourceSymbol]
  );

  // Get text color based on tile properties
  const getTextColor = useCallback(
    (tile: GameTile): string => {
      if (debug.showStats) {
        return '#ffff00'; // Yellow for debug
      }

      // Color based on terrain for better contrast
      switch (tile.terrain) {
        case TerrainType.Ocean:
          return '#ffffff';
        case TerrainType.Desert:
        case TerrainType.Snow:
          return '#333333';
        case TerrainType.Forest:
        case TerrainType.Jungle:
          return '#ffffe0';
        default:
          return '#ffffff';
      }
    },
    [debug.showStats]
  );

  // Update text elements based on camera position and tiles
  const updateTextElements = useCallback(() => {
    if (!camera) return;

    const cameraPosition = camera.position;
    const textElements = textElementsRef.current;
    const currentTileIds = new Set(tiles.map(t => t.id));

    // Clear removed tiles
    for (const [tileId] of textElements) {
      if (!currentTileIds.has(tileId)) {
        textElements.delete(tileId);
      }
    }

    // Update or create text elements for visible tiles
    for (const tile of tiles) {
      const [x, z] = HexUtils.hexToPixel(tile.hex);
      const position = new Vector3(x, tile.elevation * 0.5 + 1, z);
      const distance = cameraPosition.distanceTo(position);

      // Distance-based culling
      const visible = distance < maxTextDistance;

      // LOD-based scaling
      const scale = Math.max(
        0.1,
        Math.min(1, (maxTextDistance - distance) / maxTextDistance)
      );

      const textContent = generateTextContent(tile);
      const textColor = getTextColor(tile);

      if (textContent) {
        textElements.set(tile.id, {
          id: `text-${tile.id}`,
          position,
          text: textContent,
          color: textColor,
          scale,
          visible,
        });
      }
    }

    lastCameraPositionRef.current.copy(cameraPosition);
  }, [camera, tiles, maxTextDistance, generateTextContent, getTextColor]);

  // Update text elements on camera movement or tile changes
  useFrame(() => {
    if (!camera) return;

    const cameraPosition = camera.position;
    const cameraMoved =
      cameraPosition.distanceTo(lastCameraPositionRef.current) > 1.0;

    if (cameraMoved || textElementsRef.current.size === 0) {
      updateTextElements();
    }
  });

  // Initialize text elements
  useEffect(() => {
    updateTextElements();
  }, [updateTextElements]);

  // Early return if no text should be shown
  if (!showCoordinates && !showResources) {
    return null;
  }

  const textElements = Array.from(textElementsRef.current.values());

  return (
    <group ref={groupRef} name='hex-text-renderer'>
      {textElements
        .filter(element => element.visible && element.text)
        .map(element => (
          <Text
            key={element.id}
            position={element.position}
            color={element.color}
            scale={element.scale}
            {...textConfig}
          >
            {element.text}
          </Text>
        ))}
    </group>
  );
};

/**
 * Lightweight hex coordinate renderer
 * Shows only hex coordinates with minimal overhead
 */
export const HexCoordinateRenderer: React.FC<{
  tiles: readonly GameTile[];
  maxDistance?: number;
}> = ({ tiles, maxDistance = 15 }) => (
  <HexTextRenderer
    tiles={tiles}
    showCoordinates
    maxTextDistance={maxDistance}
    fontSize={0.3}
  />
);

/**
 * Resource information renderer
 * Shows resource symbols on tiles
 */
export const HexResourceRenderer: React.FC<{
  tiles: readonly GameTile[];
  maxDistance?: number;
}> = ({ tiles, maxDistance = 25 }) => (
  <HexTextRenderer
    tiles={tiles}
    showResources
    maxTextDistance={maxDistance}
    fontSize={0.4}
  />
);
