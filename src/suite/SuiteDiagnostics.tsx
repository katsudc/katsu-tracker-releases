import { Diagnostics } from "../features/Diagnostics";
import { DiagnosticReport } from "./DiagnosticReport";
export function SuiteDiagnostics({onSpike}:{onSpike:()=>void}){return <><Diagnostics onSpike={onSpike}/><DiagnosticReport/></>;}
