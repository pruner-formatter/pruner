{}: let
  embeddedTs =
    # typescript
    ''
      import {  spawn  } from "bun"
      
      const proc = spawn(["/bin/foo", ...args], {
        stdio: ["inherit", "inherit", "inherit"],
      });
    '';
