import claudeCodeLogo from "../assets/providers/claude-code.png";
import codexLogo from "../assets/providers/codex.png";
import type { SessionProvider } from "../types";

const providerLogos: Record<SessionProvider, string> = {
  claude: claudeCodeLogo,
  codex: codexLogo,
};

interface ProviderLogoProps {
  className?: string;
  provider: SessionProvider;
}

export function ProviderLogo({ className, provider }: ProviderLogoProps) {
  const classes = ["provider-artwork", className].filter(Boolean).join(" ");

  return (
    <img
      className={classes}
      src={providerLogos[provider]}
      alt=""
      aria-hidden="true"
      draggable={false}
    />
  );
}
