"""Две модели: текстовая (абзацы и вопросы) и картинка↔текст (фото и запросы словами)."""
import numpy as np
import torch

DEVICE = "mps" if torch.backends.mps.is_available() else "cpu"


class TextModel:
    def __init__(self, name):
        from sentence_transformers import SentenceTransformer
        self.name = name
        self.m = SentenceTransformer(name, device=DEVICE)
        self.m.max_seq_length = 512
        # e5 просит приставки, остальные (bge-m3) — нет
        self.qp, self.dp = ("query: ", "passage: ") if "e5" in name else ("", "")

    def encode_docs(self, texts):
        return self.m.encode([self.dp + t for t in texts], batch_size=16, normalize_embeddings=True,
                             show_progress_bar=len(texts) > 64)

    def encode_queries(self, texts):
        return self.m.encode([self.qp + t for t in texts], normalize_embeddings=True)


class ImageModel:
    """SigLIP 2: общее пространство для картинок и подписей на многих языках."""

    def __init__(self, name):
        from transformers import AutoModel, AutoProcessor
        self.name = name
        self.proc = AutoProcessor.from_pretrained(name)
        self.m = AutoModel.from_pretrained(name, dtype=torch.float16 if DEVICE == "mps" else torch.float32).to(DEVICE).eval()

    @torch.no_grad()
    def encode_images(self, paths, batch=16):
        from PIL import Image
        try:
            import pillow_heif
            pillow_heif.register_heif_opener()
        except ImportError:
            pass
        out = []
        for i in range(0, len(paths), batch):
            ims = [Image.open(p).convert("RGB") for p in paths[i:i + batch]]
            x = self.proc(images=ims, return_tensors="pt").to(DEVICE)
            f = self.m.get_image_features(pixel_values=x["pixel_values"].to(self.m.dtype))
            f = f.pooler_output if hasattr(f, "pooler_output") else f
            f = torch.nn.functional.normalize(f.float(), dim=-1)
            out.append(f.cpu().numpy())
            if (i // batch) % 10 == 0:
                print(f"\rкартинки {min(i + batch, len(paths))}/{len(paths)}", end="", flush=True)
        print()
        return np.concatenate(out)

    @torch.no_grad()
    def encode_queries(self, texts):
        # SigLIP 2 обучен на подписях в нижнем регистре, длина 64 знака-кусочка
        x = self.proc(text=[t.lower() for t in texts], padding="max_length", max_length=64,
                      truncation=True, return_tensors="pt").to(DEVICE)
        f = self.m.get_text_features(input_ids=x["input_ids"])
        f = f.pooler_output if hasattr(f, "pooler_output") else f
        f = torch.nn.functional.normalize(f.float(), dim=-1)
        return f.cpu().numpy()
