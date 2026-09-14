/** @type {import('next').NextConfig} */
const nextConfig = {
  // @messagebirds/sdk is consumed as TypeScript source via the npm
  // workspace symlink (no separate build step) — Next needs to run it
  // through its own compiler like first-party code.
  transpilePackages: ["@messagebirds/sdk"],
};

export default nextConfig;
