import { useEffect } from "react";

const DARK_MEDIA = "(prefers-color-scheme: dark)";

/**
 * 监听系统明暗主题，给 <html> 切换 dark class，让 HeroUI 主题与 Tailwind 的 dark 变量跟随系统。
 * HeroUI v3 通过 .dark / [data-theme="dark"] 触发暗色，这里用 class 方式。
 */
export function useSystemTheme(): void {
  useEffect(() => {
    const media = window.matchMedia(DARK_MEDIA);
    const apply = () => {
      document.documentElement.classList.toggle("dark", media.matches);
    };
    apply();
    media.addEventListener("change", apply);
    return () => media.removeEventListener("change", apply);
  }, []);
}
