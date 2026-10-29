import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "../suite/ipc";
export interface GameBarSetupReport {
  schema: number; stage: string; code: string; hresult: string | null; activity_id: string | null;
  exit_code: number | null; runtime_present: boolean | null; publisher_trusted: boolean | null;
}
export function useGameBarSetupReport() {
  const [report,setReport]=useState<GameBarSetupReport|null>(null);
  useEffect(()=>{let live=true,changed=false;let off:(()=>void)|undefined;
    void listen<GameBarSetupReport>("game-bar-setup-report",e=>{changed=true;if(live)setReport(e.payload);}).then(async unlisten=>{
      if(!live){unlisten();return;}off=unlisten;const value=await invoke<GameBarSetupReport|null>("get_game_bar_setup_report");if(live&&!changed)setReport(value);
    }).catch(()=>{});return()=>{live=false;off?.();};
  },[]);
  return report;
}
export const setupSucceeded=(code:string)=>["install_started","widget_installed","preflight_ok","publisher_trusted","helper_completed"].includes(code);
export function setupErrorKey(code:string|null):string {
  switch(code){
    case "setup_cancelled":return "spike.setupCancelled";
    case "administrator_required":case "certificate_registration_failed":return "spike.administratorRequired";
    case "dependency_package_invalid":case "widget_package_invalid":case "package_validation_failed":case "package_signature_invalid":case "publisher_mismatch":return "spike.widgetPackageInvalid";
    case "widget_package_missing":return "spike.widgetPackageMissing";
    case "store_required":case "game_bar_install":return "spike.gameBarInstallError";
    case "runtime_missing":case "dependency_missing":return "spike.runtimeMissing";
    case "deployment_policy":return "spike.installPolicyBlocked";
    case "publisher_untrusted":return "spike.publisherUntrusted";
    case "certificate_expired":return "spike.publisherExpired";
    case "package_in_use":return "spike.widgetInUse";
    case "access_denied":return "spike.installAccessDenied";
    case "powershell_unavailable":case "helper_failed":return "spike.helperFailed";
    case "package_query":case "package_query_failed":return "spike.packageQueryFailed";
    case "windows_unsupported":return "spike.windowsUnsupported";
    case "widget_registration_missing":return "spike.widgetRegistrationMissing";
    default:return "spike.outputSetupError";
  }
}
