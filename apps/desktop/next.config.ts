import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  output: "export",
  reactStrictMode: true,
  transpilePackages: ["@qbit/ui", "@qbit/ipc", "@qbit/i18n", "md3-next"],
};

export default nextConfig;
