import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useGarageBandAutoBounceQueue } from "@jamtools/slingshot/react";

function App() {
  const [projectPath, setProjectPath] = useState("");

  const bounceQueue = useGarageBandAutoBounceQueue({
    invoke,
    onProjectBounced: (projectPath, bouncePath) => {
      console.log("Bounced", projectPath, "->", bouncePath);
    },
  });

  return (
    <main style={{ padding: "2rem", fontFamily: "sans-serif" }}>
      <h1>Slingshot</h1>
      <p>GarageBand available: {bounceQueue.garageBandAvailable === null ? "checking…" : String(bounceQueue.garageBandAvailable)}</p>

      <div style={{ display: "flex", gap: "0.5rem", marginBottom: "1rem" }}>
        <input
          value={projectPath}
          onChange={(e) => setProjectPath(e.target.value)}
          placeholder="/path/to/project.band"
          style={{ flex: 1, padding: "0.4rem" }}
        />
        <button onClick={() => bounceQueue.enqueue([projectPath])}>Enqueue</button>
        <button onClick={() => bounceQueue.runNow(projectPath)}>Bounce now</button>
      </div>

      <div>
        <strong>Queue ({bounceQueue.queue.length})</strong>
        {bounceQueue.activeQueueProjectPath && (
          <p>Running: {bounceQueue.activeQueueProjectPath}</p>
        )}
        {Object.entries(bounceQueue.queueStatusByPath).map(([path, status]) => (
          <div key={path}>
            {path}: <strong>{status}</strong>
            {bounceQueue.queueErrorByPath[path] && (
              <span style={{ color: "red" }}> — {bounceQueue.queueErrorByPath[path]}</span>
            )}
          </div>
        ))}
      </div>
    </main>
  );
}

export default App;
