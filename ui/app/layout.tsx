import type { ReactNode } from "react";
import "./globals.css";

export const metadata = {
  title: "MessageBirds",
  description: "Profile timeline viewer",
};

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en">
      <body>
        <header>
          <h1>
            <a href="/">MessageBirds</a>
          </h1>
          <p className="muted">Profile timeline viewer</p>
        </header>
        {children}
      </body>
    </html>
  );
}
