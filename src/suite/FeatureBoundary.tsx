import { Component, type ReactNode } from "react";
import { useI18n } from "../lib/i18n";
function Failed({ retry }: { retry: () => void }) { const { t } = useI18n(); return <div className="personal-empty"><p>{t("suite.featureError")}</p><button className="personal-quiet" onClick={retry}>{t("suite.retry")}</button></div>; }
export class FeatureBoundary extends Component<{ children: ReactNode }, { failed: boolean }> {
  state = { failed: false };
  static getDerivedStateFromError() { return { failed: true }; }
  componentDidCatch() { console.warn("companion_section_unavailable"); }
  render() { return this.state.failed ? <Failed retry={() => this.setState({ failed: false })}/> : this.props.children; }
}
