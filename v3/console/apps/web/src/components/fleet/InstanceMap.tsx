import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import type { GeoPin } from "@/types/fleet";

// Simple equirectangular projection: map lat/lon to SVG coordinates
function project(lat: number, lon: number, width: number, height: number) {
  const x = ((lon + 180) / 360) * width;
  const y = ((90 - lat) / 180) * height;
  return { x, y };
}

interface PinProps {
  pin: GeoPin;
  width: number;
  height: number;
}

function MapPin({ pin, width, height }: PinProps) {
  const { x, y } = project(pin.lat, pin.lon, width, height);
  const isHealthy = (pin.statuses["RUNNING"] ?? 0) === pin.count;
  const hasError = (pin.statuses["ERROR"] ?? 0) > 0;

  const color = hasError ? "#ef4444" : isHealthy ? "#10b981" : "#f59e0b";
  const radius = Math.min(4 + pin.count * 1.5, 12);

  return (
    <g>
      <title>
        {pin.label} — {pin.count} instance{pin.count !== 1 ? "s" : ""}
        {pin.statuses["RUNNING"] ? `, ${pin.statuses["RUNNING"]} running` : ""}
        {pin.statuses["ERROR"] ? `, ${pin.statuses["ERROR"]} error` : ""}
      </title>
      <circle cx={x} cy={y} r={radius + 2} fill={color} fillOpacity={0.2} />
      <circle cx={x} cy={y} r={radius} fill={color} fillOpacity={0.85} />
      <text
        x={x}
        y={y + 1}
        textAnchor="middle"
        dominantBaseline="middle"
        fontSize={radius > 6 ? 8 : 6}
        fill="white"
        fontWeight="600"
      >
        {pin.count}
      </text>
    </g>
  );
}

interface InstanceMapProps {
  pins: GeoPin[];
  loading?: boolean;
}

// Minimal world map outline as a simplified SVG path (Robinson-like approximation)
// We use a simple land-mass path; the equirectangular projection maps directly to SVG coords
const WORLD_LAND_PATH = `
  M 178,12 L 185,10 L 193,10 L 200,12 L 200,15 L 195,18 L 185,18 L 178,15 Z
  M 10,48 L 45,45 L 75,42 L 95,45 L 100,55 L 85,65 L 70,70 L 55,68 L 35,65 L 15,60 Z
  M 100,55 L 140,50 L 155,48 L 165,52 L 168,60 L 155,68 L 140,72 L 120,70 L 105,65 Z
  M 48,30 L 65,25 L 80,27 L 85,35 L 75,42 L 60,40 L 48,35 Z
  M 140,25 L 165,20 L 185,22 L 195,30 L 188,38 L 170,40 L 150,38 L 140,32 Z
  M 170,55 L 195,52 L 210,55 L 215,65 L 205,72 L 185,72 L 170,65 Z
  M 155,95 L 170,90 L 180,95 L 182,108 L 172,115 L 158,112 Z
  M 90,80 L 105,78 L 108,90 L 100,98 L 88,95 Z
  M 200,80 L 230,75 L 240,82 L 235,92 L 215,95 L 200,90 Z
  M 215,50 L 240,48 L 255,55 L 255,65 L 240,70 L 220,68 Z
  M 250,48 L 275,45 L 295,48 L 300,55 L 285,62 L 265,60 L 250,55 Z
  M 295,55 L 330,52 L 350,58 L 345,70 L 320,72 L 295,65 Z
`;

export function InstanceMap({ pins, loading }: InstanceMapProps) {
  const width = 360;
  const height = 180;

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-sm font-medium">Instance Locations</CardTitle>
      </CardHeader>
      <CardContent>
        {loading ? (
          <div className="h-48 bg-muted animate-pulse rounded-md" />
        ) : (
          <div className="relative w-full overflow-hidden rounded-md bg-blue-950/20 border border-border">
            <svg
              viewBox={`0 0 ${width} ${height}`}
              className="w-full h-auto"
              style={{ display: "block" }}
              aria-label="World map with instance location pins"
            >
              {/* Ocean background */}
              <rect width={width} height={height} fill="transparent" />

              {/* Simplified continent outlines */}
              <path
                d={WORLD_LAND_PATH}
                fill="currentColor"
                className="text-muted/30"
                stroke="currentColor"
                strokeWidth={0.3}
                strokeOpacity={0.4}
              />

              {/* Grid lines */}
              {[-60, -30, 0, 30, 60].map((lat) => {
                const { y } = project(lat, 0, width, height);
                return (
                  <line
                    key={lat}
                    x1={0}
                    y1={y}
                    x2={width}
                    y2={y}
                    stroke="currentColor"
                    strokeOpacity={0.08}
                    strokeWidth={0.5}
                  />
                );
              })}
              {[-120, -60, 0, 60, 120].map((lon) => {
                const { x } = project(0, lon, width, height);
                return (
                  <line
                    key={lon}
                    x1={x}
                    y1={0}
                    x2={x}
                    y2={height}
                    stroke="currentColor"
                    strokeOpacity={0.08}
                    strokeWidth={0.5}
                  />
                );
              })}

              {/* Instance pins */}
              {pins.map((pin) => (
                <MapPin key={pin.region} pin={pin} width={width} height={height} />
              ))}
            </svg>

            {/* Legend */}
            <div className="absolute bottom-2 right-2 flex gap-3 text-xs text-muted-foreground bg-background/80 backdrop-blur-sm rounded px-2 py-1">
              <span className="flex items-center gap-1">
                <span className="inline-block w-2 h-2 rounded-full bg-emerald-500" />
                Running
              </span>
              <span className="flex items-center gap-1">
                <span className="inline-block w-2 h-2 rounded-full bg-amber-500" />
                Mixed
              </span>
              <span className="flex items-center gap-1">
                <span className="inline-block w-2 h-2 rounded-full bg-red-500" />
                Error
              </span>
            </div>

            {pins.length === 0 && (
              <div className="absolute inset-0 flex items-center justify-center text-sm text-muted-foreground">
                No geo-located instances
              </div>
            )}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
