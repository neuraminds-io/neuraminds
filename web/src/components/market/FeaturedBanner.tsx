"use client";

import { useMemo } from "react";

export interface SignalChartProps {
  label?: string;
  latency?: string;
}

function generateSignalData(points: number, seed: number): number[] {
  const data: number[] = [];
  let current = 50;
  for (let i = 0; i < points; i++) {
    const noise =
      Math.sin(i * 0.4 + seed) * 8 +
      Math.cos(i * 0.15 + seed * 2) * 12 +
      Math.sin(i * 0.08 + seed * 0.5) * 6;
    current += noise * 0.12;
    current = Math.max(15, Math.min(85, current));
    data.push(current);
  }
  return data;
}

export function SignalChart({
  label = "ORACLE_A",
  latency = "4MS",
}: SignalChartProps) {
  const signalData = useMemo(() => generateSignalData(120, 42), []);

  const width = 1000;
  const height = 200;
  const padding = { top: 10, right: 10, bottom: 10, left: 10 };
  const chartWidth = width - padding.left - padding.right;
  const chartHeight = height - padding.top - padding.bottom;

  const toPath = (data: number[]) => {
    return data
      .map((val, i) => {
        const x = padding.left + (i / (data.length - 1)) * chartWidth;
        const y = padding.top + chartHeight - (val / 100) * chartHeight;
        return `${i === 0 ? "M" : "L"} ${x} ${y}`;
      })
      .join(" ");
  };

  const gridLines = [25, 50, 75];

  return (
    <div className="border border-border bg-bg-primary/80 relative overflow-hidden">
      <div className="p-4 sm:p-5">
        <div className="flex items-center gap-6 mb-4">
          <span className="text-xs uppercase tracking-[0.16em] text-text-secondary font-mono">
            SIGNAL_INPUT: {label}
          </span>
          <span className="text-xs uppercase tracking-[0.16em] text-text-muted font-mono">
            LATENCY: {latency}
          </span>
        </div>
        <div className="h-[180px] sm:h-[220px]">
          <svg
            viewBox={`0 0 ${width} ${height}`}
            className="w-full h-full"
            preserveAspectRatio="none"
          >
            {gridLines.map((val) => (
              <line
                key={val}
                x1={padding.left}
                y1={padding.top + chartHeight - (val / 100) * chartHeight}
                x2={width - padding.right}
                y2={padding.top + chartHeight - (val / 100) * chartHeight}
                stroke="currentColor"
                strokeOpacity={0.06}
              />
            ))}
            <path
              d={toPath(signalData)}
              fill="none"
              stroke="var(--color-accent)"
              strokeWidth={2.5}
            />
          </svg>
        </div>
      </div>
    </div>
  );
}

// Keep the named export for backwards compatibility
export function FeaturedBanner() {
  return <SignalChart />;
}
