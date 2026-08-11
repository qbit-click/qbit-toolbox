"use client";

import {
  AppBar,
  Box,
  ToggleButton,
  ToggleButtonGroup,
  Toolbar,
  Typography,
} from "@mui/material";
import { useI18n } from "@qbit/i18n";
import { useMD3 } from "md3-next";
import { type ReactNode } from "react";

export interface AppShellProps {
  children: ReactNode;
  title?: string;
}

export function ThemeModeControl() {
  const { preference, setMode } = useMD3();
  const { t } = useI18n();

  return (
    <ToggleButtonGroup
      aria-label={t("theme.preference")}
      exclusive
      size="small"
      value={preference}
      onChange={(_, value) => {
        if (value !== null) setMode(value);
      }}
    >
      <ToggleButton value="system">{t("theme.system")}</ToggleButton>
      <ToggleButton value="light">{t("theme.light")}</ToggleButton>
      <ToggleButton value="dark">{t("theme.dark")}</ToggleButton>
    </ToggleButtonGroup>
  );
}

export function AppShell({ children, title }: AppShellProps) {
  const { t } = useI18n();
  return (
    <Box
      sx={{
        minHeight: "100vh",
        bgcolor: "background.default",
        color: "text.primary",
      }}
    >
      <AppBar component="header" position="sticky">
        <Toolbar>
          <Typography component="h1" variant="h6" sx={{ flexGrow: 1 }}>
            {title ?? t("app.title")}
          </Typography>
          <ThemeModeControl />
        </Toolbar>
      </AppBar>
      <Box
        component="main"
        sx={{ mx: "auto", maxWidth: 960, p: { xs: 3, sm: 5 } }}
      >
        {children}
      </Box>
    </Box>
  );
}
