import {PropsWithChildren, ReactElement, useEffect} from "react";
import {useLocalStorage} from "usehooks-ts";
import {DarkModeOption, SettingsContext} from "@/contexts/settings-context.tsx";

export function SettingsProvider({children}: PropsWithChildren): ReactElement {
  const [darkMode, setDarkMode] = useLocalStorage('theme', DarkModeOption.System);

  // sync state with localStorage
  useEffect(() => {
    if (
      darkMode === DarkModeOption.Dark ||
      (darkMode === DarkModeOption.System &&
        window.matchMedia('(prefers-color-scheme: dark)').matches)
    ) {
      document.documentElement.classList.add('dark');
    } else {
      document.documentElement.classList.remove('dark');
    }
  }, [darkMode]);

  return (
    <SettingsContext.Provider value={{darkMode, setDarkMode}}>{children}</SettingsContext.Provider>
  );
}
