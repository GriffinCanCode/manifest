import React, { Suspense } from 'react'
import { Canvas } from '@react-three/fiber'
import { OrbitControls, Environment, Grid } from '@react-three/drei'

// Placeholder 3D scene component
const GameScene: React.FC = () => {
  return (
    <>
      {/* Lighting */}
      <ambientLight intensity={0.4} />
      <directionalLight position={[10, 10, 5]} intensity={1} />
      
      {/* Placeholder terrain */}
      <mesh position={[0, -0.5, 0]} rotation={[-Math.PI / 2, 0, 0]}>
        <planeGeometry args={[20, 20]} />
        <meshStandardMaterial color="#2d5a27" />
      </mesh>
      
      {/* Grid helper */}
      <Grid 
        args={[20, 20]} 
        position={[0, 0, 0]}
        cellColor="#ffffff"
        sectionColor="#666666"
        fadeDistance={25}
        fadeStrength={1}
      />
      
      {/* Camera controls */}
      <OrbitControls 
        enablePan={true}
        enableZoom={true}
        enableRotate={true}
        minDistance={5}
        maxDistance={50}
        minPolarAngle={Math.PI / 6}
        maxPolarAngle={Math.PI / 2.5}
      />
      
      {/* Environment */}
      <Environment preset="dawn" />
    </>
  )
}

const GameCanvas: React.FC = () => {
  return (
    <div className="game-canvas">
      <Canvas
        camera={{ 
          position: [15, 10, 15], 
          fov: 60,
          near: 0.1,
          far: 1000
        }}
        shadows
        dpr={[1, 2]}
        gl={{ antialias: true, alpha: false }}
      >
        <Suspense fallback={null}>
          <GameScene />
        </Suspense>
      </Canvas>
      
      <style jsx>{`
        .game-canvas {
          width: 100%;
          height: 100%;
          position: relative;
          background: linear-gradient(to bottom, #87CEEB 0%, #98D8E8 50%, #B0E0E6 100%);
        }
      `}</style>
    </div>
  )
}

export default GameCanvas
