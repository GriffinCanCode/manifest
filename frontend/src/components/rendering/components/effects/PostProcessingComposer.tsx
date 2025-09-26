/**
 * Enhanced Post-Processing Composer integrated with RenderPipeline
 * Comprehensive post-processing pipeline using @react-three/postprocessing
 */

import { useFrame } from '@react-three/fiber';
import {
  Bloom,
  ChromaticAberration,
  EffectComposer,
  FXAA,
  N8AO,
  ToneMapping,
  Vignette,
} from '@react-three/postprocessing';
import { BlendFunction, KernelSize, ToneMappingMode } from 'postprocessing';
import React, { useMemo, useRef } from 'react';
import { Vector2 } from 'three';

import { useRenderStore } from '../../../../stores/render-store';

import { ShadowCascadeRenderer } from './ShadowCascadeRenderer';

interface PostProcessingComposerProps {
  children?: React.ReactNode;
  enabled?: boolean;
  enableTAA?: boolean;
  enableSelectiveBloom?: boolean;
}

/**
 * Enhanced post-processing composer with pipeline integration
 */
export const PostProcessingComposer: React.FC<PostProcessingComposerProps> = ({
  children,
  enabled = true,
  enableTAA = true,
  enableSelectiveBloom = false,
}) => {
  const { postprocessing, quality, capabilities, isInitialized, shadows } =
    useRenderStore();

  const composerRef = useRef<React.ElementRef<typeof EffectComposer> | null>(
    null
  );
  const frameCount = useRef(0);

  // Adaptive quality settings
  const adaptiveSettings = useMemo(() => {
    const gpuTier = capabilities?.gpuTier ?? 'medium';

    return {
      kernelSize:
        {
          low: KernelSize.VERY_SMALL,
          medium: KernelSize.SMALL,
          high: KernelSize.MEDIUM,
          ultra: KernelSize.LARGE,
        }[quality.level] ?? KernelSize.MEDIUM,

      samples: gpuTier === 'high' ? 8 : gpuTier === 'medium' ? 4 : 2,

      effectIntensities: {
        bloom: quality.level === 'low' ? 0.3 : 0.5,
        aoRadius:
          quality.level === 'low'
            ? 0.5
            : quality.level === 'medium'
              ? 0.75
              : 1.0,
        aoIntensity: quality.level === 'low' ? 0.8 : 1.2,
        taa: quality.level === 'low' ? 0.5 : 0.8,
      },
    };
  }, [quality.level, capabilities?.gpuTier]);

  // Effect enable/disable logic
  const effectsEnabled = useMemo(
    () => ({
      ssao: postprocessing.ssao && capabilities?.supportsFloatTextures,
      bloom: postprocessing.bloom && capabilities?.supportsHDR,
      toneMapping: capabilities?.supportsHDR,
      fxaa: postprocessing.fxaa && quality.antialias && !enableTAA,
      taa: enableTAA && quality.level !== 'low',
      vignette: quality.level !== 'low',
      chromaticAberration: quality.level === 'ultra',
    }),
    [postprocessing, quality, capabilities, enableTAA]
  );

  // Frame tracking for TAA
  useFrame(() => {
    frameCount.current++;
  });

  if (!isInitialized || !enabled || !postprocessing.enabled) {
    return children ? (children as React.ReactElement) : null;
  }

  return (
    <ShadowCascadeRenderer
      enabled={shadows.enabled && capabilities?.supportsShadows}
      cascades={shadows.cascades}
      shadowMapSize={shadows.mapSize}
      maxFar={shadows.maxDistance}
      shadowBias={shadows.bias}
    >
      <EffectComposer
        ref={composerRef}
        multisampling={adaptiveSettings.samples}
        depthBuffer
        enabled={enabled && postprocessing.enabled}
        enableNormalPass
      >
        {
          [
            children as React.ReactElement,

            /* Screen Space Ambient Occlusion */
            effectsEnabled.ssao && (
              <N8AO
                key='ssao'
                aoRadius={adaptiveSettings.effectIntensities.aoRadius}
                distanceFalloff={quality.level === 'low' ? 0.5 : 1.0}
                intensity={adaptiveSettings.effectIntensities.aoIntensity}
                quality={
                  quality.level === 'low'
                    ? 'low'
                    : quality.level === 'medium'
                      ? 'medium'
                      : 'high'
                }
                halfRes={quality.level === 'low'}
                screenSpaceRadius={quality.level !== 'low'}
                color='#000000'
                aoSamples={quality.level === 'low' ? 16 : 32}
                denoiseSamples={quality.level === 'low' ? 4 : 8}
              />
            ),

            /* Bloom Effect */
            effectsEnabled.bloom && (
              <Bloom
                key='bloom'
                intensity={adaptiveSettings.effectIntensities.bloom}
                luminanceThreshold={quality.level === 'low' ? 1.1 : 0.9}
                luminanceSmoothing={quality.level === 'low' ? 0.025 : 0.05}
                mipmapBlur={quality.level !== 'low'}
                kernelSize={adaptiveSettings.kernelSize}
                blendFunction={BlendFunction.ADD}
                // Selective bloom for performance
                {...(enableSelectiveBloom && {
                  luminanceThreshold: 1.5,
                  intensity: 0.8,
                })}
              />
            ),

            /* HDR Tone Mapping */
            effectsEnabled.toneMapping && (
              <ToneMapping
                key='toneMapping'
                mode={ToneMappingMode.ACES_FILMIC}
                whitePoint={16.0}
                middleGrey={0.6}
                minLuminance={0.01}
                averageLuminance={1.0}
                adaptationRate={quality.level === 'low' ? 2.0 : 1.0}
                blendFunction={BlendFunction.NORMAL}
              />
            ),

            /* Fast Approximate Anti-Aliasing */
            effectsEnabled.fxaa && (
              // eslint-disable-next-line react/jsx-pascal-case
              <FXAA
                key='fxaa'
                blendFunction={BlendFunction.NORMAL}
                // Enhanced quality for higher settings
                {...(quality.level === 'ultra' && {
                  edgeThresholdMin: 0.0312,
                  edgeThreshold: 0.063,
                })}
              />
            ),

            /* Vignette Effect */
            effectsEnabled.vignette && (
              <Vignette
                key='vignette'
                offset={quality.level === 'medium' ? 0.4 : 0.3}
                darkness={quality.level === 'low' ? 0.05 : 0.1}
                eskil={false}
                blendFunction={BlendFunction.MULTIPLY}
              />
            ),

            /* Chromatic Aberration (Ultra only) */
            effectsEnabled.chromaticAberration && (
              <ChromaticAberration
                key='chromaticAberration'
                offset={new Vector2(0.0008, 0.0012)}
                blendFunction={BlendFunction.NORMAL}
                radialModulation={false}
                modulationOffset={0.15}
              />
            ),
          ].filter(Boolean) as React.ReactElement[]
        }
      </EffectComposer>
    </ShadowCascadeRenderer>
  );
};

export default PostProcessingComposer;
