import {createContext} from 'react';

export enum DarkModeOption {
  Dark = 'dark',
  Light = 'light',
  System = 'system',
}

export type SettingsContextValue = {
  darkMode: DarkModeOption;
  setDarkMode: (darkMode: DarkModeOption) => void;
};

export const SettingsContext = createContext<SettingsContextValue>({
  darkMode: DarkModeOption.System,
  setDarkMode: () => null,
});
