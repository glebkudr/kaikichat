# Decentralized chat — animation

22 seconds, 1920×1080, 30 fps, English-only labels on the diagram, no sound. The whole animation is sped up exactly twofold relative to the first version: time positions, durations and stagger intervals are divided by 2 while building the timeline. Headings, explanations and the Sender/Recipient captions were removed.

The base illustration was created with the built-in `image_gen`; the exact prompt is kept in `image-prompt.txt`, the image in `assets/network-base.png`. The animation is written with HyperFrames / GSAP; all visual dependencies are local.

Scenario: the message is encrypted on the sender's device; three copies travel through different channels; after the first failure a stored copy moves to another node; after the second failure the next node stores and resends it; the recipient returns to the network, receives and decrypts the message. This is a conceptual illustration, not a claim about any specific protocol implementation. Successful delivery needs a stored copy and an available path to the recipient.

## Verification and export

From `/Users/glebk/Code/chat`:

```sh
python3 scripts/build-storage.py run npx --yes hyperframes@0.8.29 check media/decentralized-chat
python3 scripts/build-storage.py run env TMPDIR=/Volumes/ChatBuild/output/decentralized-chat/tmp HYPERFRAMES_EXTRACT_CACHE_DIR=/Volumes/ChatBuild/cache/hyperframes-extract npx --yes hyperframes@0.8.29 render media/decentralized-chat --output /Users/glebk/Code/chat/output/decentralized-chat/decentralized-chat-en-2x.mp4 --fps 30 --quality high --workers 2
```

HyperFrames 0.8.29 is the last version allowed by the publish-date restriction of the current npm at creation time. GSAP and MotionPathPlugin 3.15.0. HyperFrames verification: lint/runtime/layout/motion with no errors or warnings, 47/47 contrast checks passed. Key frames were reviewed visually together with the base illustration; overlaps of the storage rings with labels were removed and the packets are attached to the route curves.

Build files, verification frames and the MP4 are stored on `/Volumes/ChatBuild` through local symlinks. The image and the editable sources live in this directory.

Independent visual review: ACCEPT. The final export is also checked against frames extracted from the MP4 (forking, storage, handover to the next node, successful delivery).

CLI update check: 0.8.31 is unavailable under the current npm publish-date restriction (ETARGET); the pin was returned to 0.8.29, checks pass.
