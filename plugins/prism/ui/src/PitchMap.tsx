import { useEffect, useMemo, useRef, useState } from "react";

type VoiceMeter = {
  midi: number;
  targetHz: number;
};

type Metering = {
  detectedHz: number;
  dryTargetHz: number;
  numVoices: number;
  voices: VoiceMeter[];
};

type EventToken = [string, number];

declare global {
  interface Window {
    __JUCE__?: {
      backend: {
        addEventListener: (
          eventId: string,
          cb: (payload: unknown) => void,
        ) => EventToken;
        removeEventListener: (token: EventToken) => void;
      };
    };
  }
}

const MIDI_LO = 36; // C2
const MIDI_HI = 84; // C6
const W = 640;
const H = 120;
const PAD_X = 28;
const PAD_Y = 18;
/** Keep last good input marker through brief metering dropouts. */
const INPUT_HOLD_MS = 120;

function hzToMidi(hz: number, ref = 440): number {
  if (!(hz > 0)) return NaN;
  return 69 + 12 * Math.log2(hz / ref);
}

function midiToY(midi: number): number {
  const t = (midi - MIDI_LO) / (MIDI_HI - MIDI_LO);
  return PAD_Y + (1 - t) * (H - PAD_Y * 2);
}

function noteLabel(midi: number): string {
  const names = [
    "C",
    "C#",
    "D",
    "D#",
    "E",
    "F",
    "F#",
    "G",
    "G#",
    "A",
    "A#",
    "B",
  ];
  const n = Math.round(midi);
  const name = names[((n % 12) + 12) % 12];
  const oct = Math.floor(n / 12) - 1;
  return `${name}${oct}`;
}

export function PitchMap() {
  const [metering, setMetering] = useState<Metering>({
    detectedHz: 0,
    dryTargetHz: 0,
    numVoices: 0,
    voices: [],
  });
  const [heldHz, setHeldHz] = useState(0);
  const lastGoodAt = useRef(0);

  useEffect(() => {
    const backend = window.__JUCE__?.backend;
    if (!backend) return;

    const token = backend.addEventListener("prismMetering", (payload) => {
      const p = payload as {
        detectedHz?: number;
        dryTargetHz?: number;
        numVoices?: number;
        voices?: VoiceMeter[];
      };
      const detectedHz = Number(p.detectedHz) || 0;
      const next: Metering = {
        detectedHz,
        dryTargetHz: Number(p.dryTargetHz) || 0,
        numVoices: Number(p.numVoices) || 0,
        voices: Array.isArray(p.voices) ? p.voices : [],
      };
      setMetering(next);

      const now = performance.now();
      if (detectedHz > 0) {
        lastGoodAt.current = now;
        setHeldHz(detectedHz);
      } else if (now - lastGoodAt.current > INPUT_HOLD_MS) {
        setHeldHz(0);
      }
    });

    return () => backend.removeEventListener(token);
  }, []);

  const displayHz = heldHz > 0 ? heldHz : metering.detectedHz;
  const inputMidi = useMemo(() => hzToMidi(displayHz), [displayHz]);
  const dryMidi = useMemo(
    () => hzToMidi(metering.dryTargetHz),
    [metering.dryTargetHz],
  );

  const inputInRange =
    Number.isFinite(inputMidi) &&
    inputMidi >= MIDI_LO - 0.5 &&
    inputMidi <= MIDI_HI + 0.5;
  const inputY = inputInRange ? midiToY(inputMidi) : H / 2;
  const inputX = PAD_X + 36;

  const guideMarks = [36, 48, 60, 72, 84];

  return (
    <section className="pitch-map" aria-label="Pitch map">
      <div className="pitch-map-header">
        <span>Pitch map</span>
        <span className="value">
          {displayHz > 0 ? `${displayHz.toFixed(1)} Hz` : "— Hz"}
          {inputInRange ? ` · ${noteLabel(inputMidi)}` : ""}
          {metering.numVoices > 0
            ? ` · ${metering.numVoices} voice${metering.numVoices === 1 ? "" : "s"}`
            : ""}
        </span>
      </div>
      <svg
        viewBox={`0 0 ${W} ${H}`}
        className="pitch-map-svg"
        role="img"
        aria-label="Input pitch mapped to MIDI voices"
      >
        <rect
          x={0}
          y={0}
          width={W}
          height={H}
          rx={8}
          className="pitch-map-bg"
        />
        {guideMarks.map((m) => {
          const y = midiToY(m);
          return (
            <g key={m}>
              <line
                x1={PAD_X}
                y1={y}
                x2={W - PAD_X}
                y2={y}
                className="pitch-guide"
              />
              <text x={8} y={y + 3} className="pitch-guide-label">
                {noteLabel(m)}
              </text>
            </g>
          );
        })}

        {inputInRange &&
          metering.voices.map((v, i) => {
            const midi = Number(v.midi);
            if (!Number.isFinite(midi)) return null;
            const clamped = Math.min(Math.max(midi, MIDI_LO), MIDI_HI);
            const y = midiToY(clamped);
            const x = PAD_X + 120 + i * 56;
            return (
              <line
                key={`guide-${i}-${midi}`}
                x1={inputX}
                y1={inputY}
                x2={x}
                y2={y}
                className="pitch-map-link"
              />
            );
          })}

        {inputInRange && (
          <g>
            <circle cx={inputX} cy={inputY} r={7} className="pitch-input" />
            <text x={inputX + 12} y={inputY + 4} className="pitch-input-label">
              in
            </text>
          </g>
        )}

        {Number.isFinite(dryMidi) &&
          dryMidi >= MIDI_LO &&
          dryMidi <= MIDI_HI && (
            <circle
              cx={inputX + 28}
              cy={midiToY(dryMidi)}
              r={4}
              className="pitch-dry"
            />
          )}

        {metering.voices.map((v, i) => {
          const midi = Number(v.midi);
          if (!Number.isFinite(midi)) return null;
          const clamped = Math.min(Math.max(midi, MIDI_LO), MIDI_HI);
          const y = midiToY(clamped);
          const x = PAD_X + 120 + i * 56;
          return (
            <g key={`voice-${i}-${midi}`}>
              <rect
                x={x - 10}
                y={y - 8}
                width={20}
                height={16}
                rx={3}
                className="pitch-voice"
              />
              <text
                x={x}
                y={y + 4}
                textAnchor="middle"
                className="pitch-voice-label"
              >
                {noteLabel(midi)}
              </text>
            </g>
          );
        })}

        {!inputInRange && metering.numVoices === 0 && (
          <text
            x={W / 2}
            y={H / 2 + 4}
            textAnchor="middle"
            className="pitch-empty"
          >
            Sing in · play MIDI notes
          </text>
        )}
      </svg>
    </section>
  );
}
