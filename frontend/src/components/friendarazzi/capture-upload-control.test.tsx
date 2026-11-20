import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { useSupportsMobileCameraCapture } from "@/lib/media-capture";
import { CaptureUploadControl } from "./capture-upload-control";

vi.mock("@/lib/media-capture", () => ({
  useSupportsMobileCameraCapture: vi.fn(),
}));

const mockUseMobileCapture = vi.mocked(useSupportsMobileCameraCapture);

describe("CaptureUploadControl", () => {
  beforeEach(() => {
    mockUseMobileCapture.mockReturnValue(false);
  });

  it("renders separate camera and gallery inputs when mobile camera capture is supported", () => {
    mockUseMobileCapture.mockReturnValue(true);

    render(<CaptureUploadControl onFileSelect={vi.fn()} />);

    const cameraInput = screen.getByLabelText("Foto aufnehmen");
    const galleryInput = screen.getByLabelText("Aus Galerie auswählen");

    expect(cameraInput).toHaveAttribute("capture", "environment");
    expect(cameraInput).toHaveAttribute("accept", "image/*");
    expect(galleryInput).not.toHaveAttribute("capture");
    expect(galleryInput).toHaveAttribute("accept", "image/*");
  });

  it("renders a drag-and-drop file picker on desktop and forwards dropped image files", () => {
    const onFileSelect = vi.fn();

    render(<CaptureUploadControl onFileSelect={onFileSelect} />);

    const dropLabel = screen.getByText("Foto auswählen oder hierher ziehen");
    const input = screen.getByLabelText(/Foto auswählen oder hierher ziehen/);
    expect(input).not.toHaveAttribute("capture");

    const photo = new File(["image-bytes"], "photo.png", { type: "image/png" });
    fireEvent.drop(dropLabel, {
      dataTransfer: { files: [photo] },
    });

    expect(onFileSelect).toHaveBeenCalledWith(photo);
  });
});
