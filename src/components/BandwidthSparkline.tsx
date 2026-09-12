import React, { useEffect, useRef } from 'react';

interface Props {
  currentDown: number;
  currentUp: number;
  maxPoints?: number;
}

export const BandwidthSparkline: React.FC<Props> = ({ currentDown, currentUp, maxPoints = 40 }) => {
  const downHistory = useRef<number[]>(new Array(maxPoints).fill(0));
  const upHistory = useRef<number[]>(new Array(maxPoints).fill(0));

  useEffect(() => {
    downHistory.current.push(currentDown);
    if (downHistory.current.length > maxPoints) downHistory.current.shift();

    upHistory.current.push(currentUp);
    if (upHistory.current.length > maxPoints) upHistory.current.shift();
  }, [currentDown, currentUp, maxPoints]);

  const maxVal = Math.max(...downHistory.current, ...upHistory.current, 1024 * 100);

  const getPoints = (history: number[]) => {
    return history
      .map((val, i) => {
        const x = (i / (maxPoints - 1)) * 100;
        const y = 100 - (val / maxVal) * 85 - 5;
        return `${x.toFixed(1)},${y.toFixed(1)}`;
      })
      .join(' ');
  };

  const downPoints = getPoints(downHistory.current);
  const upPoints = getPoints(upHistory.current);

  return (
    <div className="relative w-full h-12 bg-dark-bg/60 rounded-lg border border-dark-border/40 p-1 overflow-hidden">
      <svg className="w-full h-full overflow-visible" viewBox="0 0 100 100" preserveAspectRatio="none">
        <defs>
          <linearGradient id="downGrad" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="#10B981" stopOpacity="0.25" />
            <stop offset="100%" stopColor="#10B981" stopOpacity="0.0" />
          </linearGradient>
          <linearGradient id="upGrad" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="#00F0FF" stopOpacity="0.25" />
            <stop offset="100%" stopColor="#00F0FF" stopOpacity="0.0" />
          </linearGradient>
        </defs>

        {/* Download Area & Line */}
        <polygon points={`0,100 ${downPoints} 100,100`} fill="url(#downGrad)" />
        <polyline fill="none" stroke="#10B981" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" points={downPoints} />

        {/* Upload Area & Line */}
        <polyline fill="none" stroke="#00F0FF" strokeWidth="1.8" strokeDasharray="3,3" strokeLinecap="round" points={upPoints} />
      </svg>
    </div>
  );
};
