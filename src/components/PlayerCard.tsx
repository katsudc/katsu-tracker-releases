import { privateName, usePrivateNames } from "../features/privacy";
import type { PlayerData } from "../lib/types";
import { CachedImage } from "./CachedImage";
import { AGENT_COLORS, RANK_TIERS, PARTY_COLORS, AUTHOR_PUUID } from "../lib/constants";
import { getLocalizedRank, useI18n } from "../lib/i18n";
import { useAssetsStore } from "../stores/assetsStore";
import { usePanelStore } from "../stores/panelStore";
import { RankIcon } from "../tracker/RankIcon";
import { AuthorCrown } from "./AuthorCrown";
import { OverlayUserMark } from "./OverlayUserMark";
import { normalizePuuid, useOverlayUsersStore } from "../stores/overlayUsersStore";
interface Props { player: PlayerData; slotIndex?: number }
export function PlayerCard({ player, slotIndex = 1 }: Props) {
  const { t, locale } = useI18n();
  const { getAgentIcon } = useAssetsStore();
  const { openPlayer, selectedPlayer, panelType } = usePanelStore();
  const overlayPuuids = useOverlayUsersStore(s => s.puuids);
  const isSelected = panelType === "player" && selectedPlayer?.puuid === player.puuid;
  const agentLabel = player.agent ? player.agent.charAt(0).toUpperCase() + player.agent.slice(1) : t("player.agentUnavailable");
  const privateNames = usePrivateNames();
  const displayName = privateNames ? privateName(player.is_me, slotIndex) : player.name?.trim() || t("player.anonymousSlot", { n: slotIndex });
  const agentIcon = player.agent ? getAgentIcon(player.agent) : null;
  const agentColor = AGENT_COLORS[player.agent?.toLowerCase()] || "#768079";
  const knownRank = player.rank_available === true || (player.rank_available == null && player.rank_tier > 0);
  const rankName = knownRank ? getLocalizedRank(player.rank_tier, locale) : t("player.rankUnavailable");
  const rankColor = knownRank ? RANK_TIERS[player.rank_tier]?.[1] || "#91a1b7" : "#8496ae";
  const partyIndex = /^(Grup|Group)-/.test(player.party) ? Number(player.party.split("-")[1]) - 1 : -1;
  const partyColor = partyIndex >= 0 ? PARTY_COLORS[partyIndex % 4] : null;
  const isAuthor = player.puuid.toLowerCase() === AUTHOR_PUUID;
  const normalized = normalizePuuid(player.puuid);
  const isOverlayUser = !isAuthor && normalized != null && overlayPuuids.includes(normalized);
  const nameTitle = player.name_from_history ? t("player.rememberedNameHint", { name: displayName }) : displayName;
  return <div className={`player-rank-card ${isSelected ? "is-selected" : ""} ${player.is_me ? "is-me" : ""}`}
    role="button" tabIndex={0} aria-label={`${displayName} · ${agentLabel} · ${rankName}`}
    onClick={() => void openPlayer(player)} onKeyDown={event => { if (event.key === "Enter" || event.key === " ") { event.preventDefault(); void openPlayer(player); } }}>
    {player.player_card_id && <CachedImage src={`https://media.valorant-api.com/playercards/${player.player_card_id}/wideart.png`} alt="" silent softOpacity={0.17} className="player-rank-banner"/>}
    {partyColor && <i className="player-party-marker" style={{ background: partyColor }}/>}
    <span className="player-agent-avatar" title={agentLabel} style={{ borderColor: `${agentColor}70` }}>
      {agentIcon ? <CachedImage src={agentIcon} alt="" className="player-agent-image"/> : <i className={player.locked ? "is-locked" : ""}/>}
    </span>
    <span className="player-identity"><span className={`player-identity-name ${player.name_from_history ? "is-remembered" : ""}`} title={nameTitle}>
      <b>{displayName}</b>{isAuthor && <AuthorCrown className="w-3 h-3 text-accent-gold" title={t("player.authorHint")}/>}{isOverlayUser && <OverlayUserMark title={t("player.overlayUserHint")}/>}
    </span><small>{agentLabel}</small></span>
    <span className="player-current-rank" title={rankName} style={{ color: rankColor }}><RankIcon tier={knownRank ? player.rank_tier : null} compact/><strong>{rankName}</strong></span>
  </div>;
}
