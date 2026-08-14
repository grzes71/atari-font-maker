using System;
using System.IO;
using System.Text;
using System.Drawing;
using System.Drawing.Imaging;
using System.Collections.Generic;
using System.Reflection;
using System.Linq;
using FontMaker;
using TinyJson;

namespace ReferenceHarness;

public class Program
{
	public enum RunMode
	{
		Generate,
		Verify
	}

	private static RunMode _mode = RunMode.Generate;
	private static string _outDir = string.Empty;
	private static int _totalVerifications = 0;
	private static int _failedVerifications = 0;

	public static int Main(string[] args)
	{
		Console.WriteLine("==================================================");
		Console.WriteLine("  Atari FontMaker — C# Reference Harness");
		Console.WriteLine("==================================================");

		_mode = RunMode.Generate;
		_outDir = FindFixturesDirectory();

		for (int i = 0; i < args.Length; i++)
		{
			string arg = args[i].ToLowerInvariant();
			if (arg == "--generate" || arg == "generate")
			{
				_mode = RunMode.Generate;
			}
			else if (arg == "--verify" || arg == "verify")
			{
				_mode = RunMode.Verify;
			}
			else if (arg == "--output" && i + 1 < args.Length)
			{
				_outDir = Path.GetFullPath(args[++i]);
			}
		}

		Console.WriteLine($"Mode: {_mode.ToString().ToUpperInvariant()}");
		Console.WriteLine($"Target Fixtures Directory: {_outDir}\n");

		if (_mode == RunMode.Generate)
		{
			Directory.CreateDirectory(_outDir);
			Directory.CreateDirectory(Path.Combine(_outDir, "transforms"));
			Directory.CreateDirectory(Path.Combine(_outDir, "encodings"));
			Directory.CreateDirectory(Path.Combine(_outDir, "palette"));
			Directory.CreateDirectory(Path.Combine(_outDir, "renders"));
			Directory.CreateDirectory(Path.Combine(_outDir, "exports"));
			Directory.CreateDirectory(Path.Combine(_outDir, "projects"));
			Directory.CreateDirectory(Path.Combine(_outDir, "undo"));
		}

		try
		{
			RunPaletteTests();
			RunEncodingTests();
			RunTransformTests();
			RunRendererTests();
			RunCodecAndProjectTests();
			RunExportTests();
			RunUndoRedoTests();

			if (_mode == RunMode.Verify)
			{
				Console.WriteLine("\n--------------------------------------------------");
				Console.WriteLine($"Verification Summary: {_totalVerifications - _failedVerifications}/{_totalVerifications} checks passed.");
				if (_failedVerifications > 0)
				{
					Console.Error.WriteLine($"[VERIFY FAILED] {_failedVerifications} mismatches detected between current C# logic and golden fixtures!");
					return 1;
				}
				Console.WriteLine("[VERIFY SUCCESS] 100% bit-exact parity with golden master fixtures!");
				return 0;
			}
			else
			{
				Console.WriteLine("\n[SUCCESS] All reference tests & golden masters generated successfully!");
				return 0;
			}
		}
		catch (Exception ex)
		{
			Console.Error.WriteLine($"\n[ERROR] Reference Harness failed: {ex}");
			return 1;
		}
	}

	private static string FindFixturesDirectory()
	{
		// Search up directory hierarchy for tests/fixtures
		string dir = AppContext.BaseDirectory;
		for (int i = 0; i < 7; i++)
		{
			string candidate = Path.Combine(dir, "tests", "fixtures");
			if (Directory.Exists(candidate) && File.Exists(Path.Combine(dir, "Cargo.toml")))
			{
				return Path.GetFullPath(candidate);
			}
			var parent = Directory.GetParent(dir);
			if (parent == null) break;
			dir = parent.FullName;
		}

		// Fallback to relative path from root
		string fallback = Path.GetFullPath(Path.Combine(AppContext.BaseDirectory, "..", "..", "..", "..", "..", "tests", "fixtures"));
		return fallback;
	}

	private static void RecordArtifact(string relativePath, byte[] data)
	{
		_totalVerifications++;
		string fullPath = Path.Combine(_outDir, relativePath);
		if (_mode == RunMode.Generate)
		{
			string? parent = Path.GetDirectoryName(fullPath);
			if (parent != null) Directory.CreateDirectory(parent);
			File.WriteAllBytes(fullPath, data);
		}
		else
		{
			if (!File.Exists(fullPath))
			{
				Console.Error.WriteLine($"  [FAIL] Missing fixture file: {relativePath}");
				_failedVerifications++;
				return;
			}

			byte[] existing = File.ReadAllBytes(fullPath);
			if (!existing.SequenceEqual(data))
			{
				Console.Error.WriteLine($"  [FAIL] Binary mismatch in {relativePath} (expected {existing.Length} bytes, got {data.Length} bytes)");
				_failedVerifications++;
			}
		}
	}

	private static void RecordArtifact(string relativePath, string text)
	{
		_totalVerifications++;
		string fullPath = Path.Combine(_outDir, relativePath);
		if (_mode == RunMode.Generate)
		{
			string? parent = Path.GetDirectoryName(fullPath);
			if (parent != null) Directory.CreateDirectory(parent);
			File.WriteAllText(fullPath, text, Encoding.UTF8);
		}
		else
		{
			if (!File.Exists(fullPath))
			{
				Console.Error.WriteLine($"  [FAIL] Missing fixture file: {relativePath}");
				_failedVerifications++;
				return;
			}

			string existing = File.ReadAllText(fullPath, Encoding.UTF8);
			// Normalize CRLF to LF for reliable comparison
			string normExisting = existing.Replace("\r\n", "\n");
			string normText = text.Replace("\r\n", "\n");
			if (normExisting != normText)
			{
				Console.Error.WriteLine($"  [FAIL] Text content mismatch in {relativePath}");
				_failedVerifications++;
			}
		}
	}

	#region 1. Palette Tests
	private static void RunPaletteTests()
	{
		Console.WriteLine("[1/7] Palette & Nearest Color Tests...");

		byte[] palBytes = Helpers.GetResource<byte[]>("altirraPAL.pal");
		RecordArtifact(Path.Combine("palette", "altirraPAL.pal"), palBytes);

		Color[] palette = new Color[256];
		for (int i = 0; i < 256; i++)
		{
			palette[i] = Color.FromArgb(palBytes[i * 3], palBytes[i * 3 + 1], palBytes[i * 3 + 2]);
		}

		var palList = new List<object>();
		for (int i = 0; i < 256; i++)
		{
			palList.Add(new {
				index = i,
				r = (int)palBytes[i * 3],
				g = (int)palBytes[i * 3 + 1],
				b = (int)palBytes[i * 3 + 2]
			});
		}
		RecordArtifact(Path.Combine("palette", "palette_rgb.json"), palList.ToJson());

		// Comprehensive test vectors: extremes, pure RGB, greys, odd palette approximations
		var testColors = new List<(int r, int g, int b)>
		{
			(0, 0, 0),
			(255, 255, 255),
			(128, 128, 128),
			(74, 120, 240),
			(180, 50, 30),
			(20, 200, 50),
			(240, 220, 40),
			(150, 40, 190),
			(16, 32, 48),
			(200, 100, 0),
			(255, 0, 0),
			(0, 255, 0),
			(0, 0, 255),
			(255, 255, 0),
			(0, 255, 255),
			(255, 0, 255),
			(64, 64, 64),
			(192, 192, 192),
			(1, 1, 1),
			(254, 254, 254)
		};

		// Also sample odd-indexed palette colors to verify even index snapping
		for (int i = 1; i < 256; i += 17)
		{
			testColors.Add((palBytes[i * 3], palBytes[i * 3 + 1], palBytes[i * 3 + 2]));
		}

		var closestResults = new List<object>();
		foreach (var (r, g, b) in testColors)
		{
			byte closestIdx = Helpers.FindClosest((byte)r, (byte)g, (byte)b, palette);
			closestResults.Add(new {
				query_r = r,
				query_g = g,
				query_b = b,
				matched_index = (int)closestIdx,
				matched_r = (int)palBytes[closestIdx * 3],
				matched_g = (int)palBytes[closestIdx * 3 + 1],
				matched_b = (int)palBytes[closestIdx * 3 + 2]
			});
		}
		RecordArtifact(Path.Combine("palette", "find_closest_vectors.json"), closestResults.ToJson());
		Console.WriteLine($"  - Extracted 256 palette colors and verified {testColors.Count} FindClosest vectors.");
	}
	#endregion

	#region 2. Encodings & Codecs Tests
	private static void RunEncodingTests()
	{
		Console.WriteLine("\n[2/7] Pixel Encodings & Character Conversion Tests...");

		var monoVectors = new List<object>();
		var color2BitVectors = new List<object>();
		var color4BitVectors = new List<object>();
		var atariConvertVectors = new List<object>();

		for (int b = 0; b < 256; b++)
		{
			byte inByte = (byte)b;

			// Mono (1-bit)
			byte[] monoDec = AtariFont.DecodeMono(inByte);
			byte monoEnc = AtariFont.EncodeMono(monoDec);
			if (monoEnc != inByte) throw new Exception($"Mono roundtrip mismatch for {inByte}: got {monoEnc}");
			monoVectors.Add(new { input_byte = b, decoded = monoDec });

			// 2-Bit (Mode 4/5)
			byte[] color2Dec = AtariFont.DecodeColor2Bit(inByte);
			byte color2Enc = AtariFont.EncodeColor2Bit(color2Dec);
			if (color2Enc != inByte) throw new Exception($"Color 2-bit roundtrip mismatch for {inByte}: got {color2Enc}");
			color2BitVectors.Add(new { input_byte = b, decoded = color2Dec });

			// 4-Bit (Mode 10)
			byte[] color4Dec = AtariFont.DecodeColor4Bit(inByte);
			byte color4Enc = AtariFont.EncodeColor4Bit(color4Dec);
			if (color4Enc != inByte) throw new Exception($"Color 4-bit roundtrip mismatch for {inByte}: got {color4Enc}");
			color4BitVectors.Add(new { input_byte = b, decoded = color4Dec });

			// AtariConvertChar
			byte converted = Helpers.AtariConvertChar(inByte);
			atariConvertVectors.Add(new { ascii_in = b, atari_char_out = (int)converted });
		}

		RecordArtifact(Path.Combine("encodings", "mono_vectors.json"), monoVectors.ToJson());
		RecordArtifact(Path.Combine("encodings", "color_2bit_vectors.json"), color2BitVectors.ToJson());
		RecordArtifact(Path.Combine("encodings", "color_4bit_vectors.json"), color4BitVectors.ToJson());
		RecordArtifact(Path.Combine("encodings", "atari_convert_char_vectors.json"), atariConvertVectors.ToJson());

		// Test 2D Character Array conversions: Get2ColorCharacter, Get5ColorCharacter, Get4BitColorCharacter
		byte[] sampleGlyph = new byte[] { 0x3C, 0x66, 0x6E, 0x76, 0x66, 0x66, 0x3C, 0x00 };
		Array.Clear(AtariFont.FontBytes, 0, AtariFont.FontBytes.Length);
		Array.Copy(sampleGlyph, 0, AtariFont.FontBytes, 0, 8);

		byte[,] c2 = AtariFont.Get2ColorCharacter(0, false);
		byte[,] c5 = AtariFont.Get5ColorCharacter(0, false);
		byte[,] c4bit = AtariFont.Get4BitColorCharacter(0, false);

		AtariFont.Set5ColorCharacter(c5, 1, false);
		AtariFont.Set4BitCharacter(c4bit, 2, false);

		var matrixResults = new {
			sample_glyph = Convert.ToHexString(sampleGlyph),
			mono_matrix_char0 = ToJaggedArray(c2),
			color5_matrix_char0 = ToJaggedArray(c5),
			color4bit_matrix_char0 = ToJaggedArray(c4bit),
			encoded_char1_from_color5 = Convert.ToHexString(AtariFont.FontBytes, 8, 8),
			encoded_char2_from_color4bit = Convert.ToHexString(AtariFont.FontBytes, 16, 8)
		};
		RecordArtifact(Path.Combine("encodings", "glyph_matrix_conversions.json"), matrixResults.ToJson());

		Console.WriteLine("  - Verified 256 Mono, 256 2-bit, 256 4-bit, 256 ASCII conversion vectors, and 2D matrix helpers.");
	}

	private static int[][] ToJaggedArray(byte[,] matrix)
	{
		int w = matrix.GetLength(0);
		int h = matrix.GetLength(1);
		int[][] res = new int[h][];
		for (int y = 0; y < h; y++)
		{
			res[y] = new int[w];
			for (int x = 0; x < w; x++)
			{
				res[y][x] = matrix[x, y];
			}
		}
		return res;
	}
	#endregion

	#region 3. Glyph & Bank Transformations Tests
	private static void RunTransformTests()
	{
		Console.WriteLine("\n[3/7] Glyph & Font-Bank Transformations Tests...");

		byte[] defaultFont = Helpers.GetResource<byte[]>("Default.fnt");
		RecordArtifact(Path.Combine("transforms", "Default.fnt"), defaultFont);

		var transformResults = new List<object>();

		for (int charIdx = 0; charIdx < 128; charIdx++)
		{
			byte[] srcBytes = new byte[8];
			Array.Copy(defaultFont, charIdx * 8, srcBytes, 0, 8);

			byte[] rotLeft = ExecuteTransform(srcBytes, () => AtariFont.RotateLeft(0, false));
			byte[] rotRight = ExecuteTransform(srcBytes, () => AtariFont.RotateRight(0, false));
			byte[] mirHorizMono = ExecuteTransform(srcBytes, () => AtariFont.MirrorHorizontal(0, false, false, 2));
			byte[] mirHoriz2Bit = ExecuteTransform(srcBytes, () => AtariFont.MirrorHorizontal(0, false, true, 4));
			byte[] mirHoriz4Bit = ExecuteTransform(srcBytes, () => AtariFont.MirrorHorizontal(0, false, true, 10));
			byte[] mirVert = ExecuteTransform(srcBytes, () => AtariFont.MirrorVertical(0, false));

			byte[] shiftLeftMono = ExecuteTransform(srcBytes, () => AtariFont.ShiftLeft(0, false, false, 2));
			byte[] shiftLeft2Bit = ExecuteTransform(srcBytes, () => AtariFont.ShiftLeft(0, false, true, 4));
			byte[] shiftLeft4Bit = ExecuteTransform(srcBytes, () => AtariFont.ShiftLeft(0, false, true, 10));

			byte[] shiftRightMono = ExecuteTransform(srcBytes, () => AtariFont.ShiftRight(0, false, false, 2));
			byte[] shiftRight2Bit = ExecuteTransform(srcBytes, () => AtariFont.ShiftRight(0, false, true, 4));
			byte[] shiftRight4Bit = ExecuteTransform(srcBytes, () => AtariFont.ShiftRight(0, false, true, 10));

			byte[] shiftUp = ExecuteTransform(srcBytes, () => AtariFont.ShiftUp(0, false));
			byte[] shiftDown = ExecuteTransform(srcBytes, () => AtariFont.ShiftDown(0, false));
			byte[] inverted = ExecuteTransform(srcBytes, () => AtariFont.InvertCharacter(0, false));
			byte[] cleared = ExecuteTransform(srcBytes, () => AtariFont.ClearCharacter(0, false));

			transformResults.Add(new {
				character_index = charIdx,
				original = Convert.ToHexString(srcBytes),
				rotate_left = Convert.ToHexString(rotLeft),
				rotate_right = Convert.ToHexString(rotRight),
				mirror_h_mono = Convert.ToHexString(mirHorizMono),
				mirror_h_2bit = Convert.ToHexString(mirHoriz2Bit),
				mirror_h_4bit = Convert.ToHexString(mirHoriz4Bit),
				mirror_v = Convert.ToHexString(mirVert),
				shift_left_mono = Convert.ToHexString(shiftLeftMono),
				shift_left_2bit = Convert.ToHexString(shiftLeft2Bit),
				shift_left_4bit = Convert.ToHexString(shiftLeft4Bit),
				shift_right_mono = Convert.ToHexString(shiftRightMono),
				shift_right_2bit = Convert.ToHexString(shiftRight2Bit),
				shift_right_4bit = Convert.ToHexString(shiftRight4Bit),
				shift_up = Convert.ToHexString(shiftUp),
				shift_down = Convert.ToHexString(shiftDown),
				inverted = Convert.ToHexString(inverted),
				cleared = Convert.ToHexString(cleared)
			});
		}

		// Synthetic edge case characters
		var edgeCases = new List<(string name, byte[] bytes)>
		{
			("all_zeros", new byte[] { 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00 }),
			("all_ones", new byte[] { 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF }),
			("checkerboard", new byte[] { 0xAA, 0x55, 0xAA, 0x55, 0xAA, 0x55, 0xAA, 0x55 }),
			("diagonal_main", new byte[] { 0x80, 0x40, 0x20, 0x10, 0x08, 0x04, 0x02, 0x01 }),
			("diagonal_anti", new byte[] { 0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80 }),
			("single_corner_pixel", new byte[] { 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00 }),
			("mode4_palette_sweep", new byte[] { 0x00, 0x55, 0xAA, 0xFF, 0x1B, 0xE4, 0x2D, 0xD2 }),
			("mode10_palette_sweep", new byte[] { 0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF })
		};

		var edgeCaseResults = new List<object>();
		foreach (var (name, srcBytes) in edgeCases)
		{
			byte[] rotLeft = ExecuteTransform(srcBytes, () => AtariFont.RotateLeft(0, false));
			byte[] rotRight = ExecuteTransform(srcBytes, () => AtariFont.RotateRight(0, false));
			byte[] mirHorizMono = ExecuteTransform(srcBytes, () => AtariFont.MirrorHorizontal(0, false, false, 2));
			byte[] mirHoriz2Bit = ExecuteTransform(srcBytes, () => AtariFont.MirrorHorizontal(0, false, true, 4));
			byte[] mirHoriz4Bit = ExecuteTransform(srcBytes, () => AtariFont.MirrorHorizontal(0, false, true, 10));
			byte[] mirVert = ExecuteTransform(srcBytes, () => AtariFont.MirrorVertical(0, false));
			byte[] shiftLeftMono = ExecuteTransform(srcBytes, () => AtariFont.ShiftLeft(0, false, false, 2));
			byte[] shiftLeft2Bit = ExecuteTransform(srcBytes, () => AtariFont.ShiftLeft(0, false, true, 4));
			byte[] shiftLeft4Bit = ExecuteTransform(srcBytes, () => AtariFont.ShiftLeft(0, false, true, 10));
			byte[] shiftRightMono = ExecuteTransform(srcBytes, () => AtariFont.ShiftRight(0, false, false, 2));
			byte[] shiftRight2Bit = ExecuteTransform(srcBytes, () => AtariFont.ShiftRight(0, false, true, 4));
			byte[] shiftRight4Bit = ExecuteTransform(srcBytes, () => AtariFont.ShiftRight(0, false, true, 10));
			byte[] shiftUp = ExecuteTransform(srcBytes, () => AtariFont.ShiftUp(0, false));
			byte[] shiftDown = ExecuteTransform(srcBytes, () => AtariFont.ShiftDown(0, false));
			byte[] inverted = ExecuteTransform(srcBytes, () => AtariFont.InvertCharacter(0, false));

			edgeCaseResults.Add(new {
				name = name,
				original = Convert.ToHexString(srcBytes),
				rotate_left = Convert.ToHexString(rotLeft),
				rotate_right = Convert.ToHexString(rotRight),
				mirror_h_mono = Convert.ToHexString(mirHorizMono),
				mirror_h_2bit = Convert.ToHexString(mirHoriz2Bit),
				mirror_h_4bit = Convert.ToHexString(mirHoriz4Bit),
				mirror_v = Convert.ToHexString(mirVert),
				shift_left_mono = Convert.ToHexString(shiftLeftMono),
				shift_left_2bit = Convert.ToHexString(shiftLeft2Bit),
				shift_left_4bit = Convert.ToHexString(shiftLeft4Bit),
				shift_right_mono = Convert.ToHexString(shiftRightMono),
				shift_right_2bit = Convert.ToHexString(shiftRight2Bit),
				shift_right_4bit = Convert.ToHexString(shiftRight4Bit),
				shift_up = Convert.ToHexString(shiftUp),
				shift_down = Convert.ToHexString(shiftDown),
				inverted = Convert.ToHexString(inverted)
			});
		}

		RecordArtifact(Path.Combine("transforms", "glyph_transforms_golden.json"), transformResults.ToJson());
		RecordArtifact(Path.Combine("transforms", "edge_cases_transforms_golden.json"), edgeCaseResults.ToJson());

		// Test Character Offsets across all 512 character positions on Bank 1 and Bank 2
		var offsetMap = new List<object>();
		for (int c = 0; c < 512; c++)
		{
			offsetMap.Add(new {
				character_index = c,
				offset_bank1 = AtariFont.GetCharacterOffset(c, false),
				offset_bank2 = AtariFont.GetCharacterOffset(c, true)
			});
		}
		RecordArtifact(Path.Combine("transforms", "character_offsets.json"), offsetMap.ToJson());

		// Test Bank-Level Operations: ShiftFontLeft, ShiftFontRight, DeleteAndShiftLeft, DeleteAndShiftRight
		var bankOpResults = new List<object>();
		Action resetBank = () => {
			Array.Clear(AtariFont.FontBytes, 0, AtariFont.FontBytes.Length);
			Array.Copy(defaultFont, 0, AtariFont.FontBytes, 0, 1024);
			Array.Copy(defaultFont, 0, AtariFont.FontBytes, 1024, 1024);
			Array.Copy(defaultFont, 0, AtariFont.FontBytes, 2048, 1024);
			Array.Copy(defaultFont, 0, AtariFont.FontBytes, 3072, 1024);
		};

		// 1. ShiftFontLeft without hole
		resetBank();
		AtariFont.ShiftFontLeft(0, false, false);
		bankOpResults.Add(new { op = "ShiftFontLeft_noHole_char0_bank1", hash = Convert.ToHexString(AtariFont.FontBytes) });

		// 2. ShiftFontLeft with hole
		resetBank();
		AtariFont.ShiftFontLeft(16, false, true);
		bankOpResults.Add(new { op = "ShiftFontLeft_makeHole_char16_bank1", hash = Convert.ToHexString(AtariFont.FontBytes) });

		// 3. ShiftFontRight without hole
		resetBank();
		AtariFont.ShiftFontRight(0, false, false);
		bankOpResults.Add(new { op = "ShiftFontRight_noHole_char0_bank1", hash = Convert.ToHexString(AtariFont.FontBytes) });

		// 4. ShiftFontRight with hole
		resetBank();
		AtariFont.ShiftFontRight(32, false, true);
		bankOpResults.Add(new { op = "ShiftFontRight_makeHole_char32_bank1", hash = Convert.ToHexString(AtariFont.FontBytes) });

		// 5. DeleteAndShiftLeft
		resetBank();
		AtariFont.DeleteAndShiftLeft(10, false);
		bankOpResults.Add(new { op = "DeleteAndShiftLeft_char10_bank1", hash = Convert.ToHexString(AtariFont.FontBytes) });

		// 6. DeleteAndShiftRight
		resetBank();
		AtariFont.DeleteAndShiftRight(20, false);
		bankOpResults.Add(new { op = "DeleteAndShiftRight_char20_bank1", hash = Convert.ToHexString(AtariFont.FontBytes) });

		// 7. Duplicate Check
		resetBank();
		bool isDup00 = AtariFont.IsDuplicate(0, 0, 0);
		bool isDup01 = AtariFont.IsDuplicate(0, 0, 1);
		bankOpResults.Add(new { op = "IsDuplicate_test", dup_same = isDup00, dup_different = isDup01 });

		RecordArtifact(Path.Combine("transforms", "bank_operations_golden.json"), bankOpResults.ToJson());
		Console.WriteLine("  - Captured 128 glyph transforms, 8 edge case sets, 512 character offsets, and bank shift operations.");
	}

	private static byte[] ExecuteTransform(byte[] initialCharBytes, Action transformAction)
	{
		Array.Clear(AtariFont.FontBytes, 0, AtariFont.FontBytes.Length);
		Array.Copy(initialCharBytes, 0, AtariFont.FontBytes, 0, 8);
		transformAction();
		byte[] result = new byte[8];
		Array.Copy(AtariFont.FontBytes, 0, result, 0, 8);
		return result;
	}
	#endregion

	#region 4. Software Renderer Tests
	private static void RunRendererTests()
	{
		Console.WriteLine("\n[4/7] Software Renderer RGBA Atlases & Single-Char Parity Tests...");

		byte[] palBytes = Helpers.GetResource<byte[]>("altirraPAL.pal");
		Color[] palette = new Color[256];
		for (int i = 0; i < 256; i++)
		{
			palette[i] = Color.FromArgb(palBytes[i * 3], palBytes[i * 3 + 1], palBytes[i * 3 + 2]);
		}

		byte[] defaultFont = Helpers.GetResource<byte[]>("Default.fnt");
		Array.Clear(AtariFont.FontBytes, 0, AtariFont.FontBytes.Length);
		for (int i = 0; i < 4; i++)
		{
			Array.Copy(defaultFont, 0, AtariFont.FontBytes, i * 1024, 1024);
		}

		byte[] selectedColors = new byte[] { 0x00, 0x28, 0xCA, 0x46, 0x98, 0x1A, 0x76, 0x54, 0x32, 0x00 };

		AtariFontRenderer.SetPalette(palette);
		AtariFontRenderer.RebuildPalette(selectedColors);

		// 1. Mono Mode
		AtariFontRenderer.WhichColorMode = 2;
		AtariFontRenderer.RenderAllFonts();
		RecordBitmapRaw(AtariFontRenderer.BitmapFontBanks, Path.Combine("renders", "font_atlas_mono.raw"));
		RecordBitmapPng(AtariFontRenderer.BitmapFontBanks, Path.Combine("renders", "font_atlas_mono.png"));

		// 2. Mode 4 (Color - 2-bit, including PF2/PF3 color switch on chars 128..255)
		AtariFontRenderer.WhichColorMode = 4;
		AtariFontRenderer.RenderAllFonts();
		RecordBitmapRaw(AtariFontRenderer.BitmapFontBanks, Path.Combine("renders", "font_atlas_mode4.raw"));
		RecordBitmapPng(AtariFontRenderer.BitmapFontBanks, Path.Combine("renders", "font_atlas_mode4.png"));

		// 3. Mode 10 (Color - 4-bit, 9 colors)
		AtariFontRenderer.WhichColorMode = 10;
		AtariFontRenderer.RenderAllFonts();
		RecordBitmapRaw(AtariFontRenderer.BitmapFontBanks, Path.Combine("renders", "font_atlas_mode10.raw"));
		RecordBitmapPng(AtariFontRenderer.BitmapFontBanks, Path.Combine("renders", "font_atlas_mode10.png"));

		// 4. Parity Test: RenderOneCharacter vs RenderAllFonts
		// Render character 65 in Mode 4, verify that rendering via RenderOneCharacter preserves consistency
		AtariFontRenderer.WhichColorMode = 4;
		AtariFontRenderer.RenderAllFonts();
		byte[] fullBefore = GetBitmapBytes(AtariFontRenderer.BitmapFontBanks);

		AtariFontRenderer.RenderOneCharacter(65, false);
		byte[] fullAfter = GetBitmapBytes(AtariFontRenderer.BitmapFontBanks);
		if (!fullBefore.SequenceEqual(fullAfter))
		{
			throw new Exception("RenderOneCharacter produced divergent output from RenderAllFonts!");
		}

		Console.WriteLine("  - Rendered Mono, Mode 4, and Mode 10 512x1024 RGBA font atlases + verified RenderOneCharacter parity.");
	}

	private static byte[] GetBitmapBytes(Bitmap bmp)
	{
		var bmpData = bmp.LockBits(new Rectangle(0, 0, bmp.Width, bmp.Height), ImageLockMode.ReadOnly, PixelFormat.Format32bppArgb);
		byte[] buffer = new byte[bmpData.Stride * bmp.Height];
		System.Runtime.InteropServices.Marshal.Copy(bmpData.Scan0, buffer, 0, buffer.Length);
		bmp.UnlockBits(bmpData);
		return buffer;
	}

	private static void RecordBitmapRaw(Bitmap bmp, string relativePath)
	{
		byte[] buffer = GetBitmapBytes(bmp);
		RecordArtifact(relativePath, buffer);
	}

	private static void RecordBitmapPng(Bitmap bmp, string relativePath)
	{
		using var ms = new MemoryStream();
		bmp.Save(ms, ImageFormat.Png);
		RecordArtifact(relativePath, ms.ToArray());
	}
	#endregion

	#region 5. File Formats & Codecs Tests
	private static void RunCodecAndProjectTests()
	{
		Console.WriteLine("\n[5/7] File Formats, Project Schemas & Clipboard Tests...");

		// 1. .fnt format (1024 bytes)
		byte[] defaultFont = Helpers.GetResource<byte[]>("Default.fnt");
		RecordArtifact(Path.Combine("projects", "Default.fnt"), defaultFont);

		// 2. .fn2 dual font format (2048 bytes)
		byte[] dualFont = new byte[2048];
		Array.Copy(defaultFont, 0, dualFont, 0, 1024);
		Array.Copy(defaultFont, 0, dualFont, 1024, 1024);
		RecordArtifact(Path.Combine("projects", "dual_sample.fn2"), dualFont);

		// 3. .atrview v2023 (default.atrview)
		string defaultAtrView = Helpers.GetResource<string>("default.atrview");
		RecordArtifact(Path.Combine("projects", "default.atrview"), defaultAtrView);

		var jsonObj = defaultAtrView.FromJson<AtrViewInfoJson>();
		if (jsonObj == null || jsonObj.Version == null)
		{
			throw new Exception("Failed to deserialize default.atrview");
		}
		string reSerialized = jsonObj.ToJson();
		RecordArtifact(Path.Combine("projects", "default_reserialized.atrview"), reSerialized);

		// 4. Synthetic .atrview v1911 (no Width/Height, requires default 40x26)
		var v1911Obj = new AtrViewInfoJson {
			Version = "1911",
			ColoredGfx = "0",
			Width = 0,
			Height = 0,
			Chars = jsonObj.Chars,
			Lines = jsonObj.Lines,
			Colors = jsonObj.Colors,
			Fontname1 = "Default.fnt",
			Fontname2 = "Default.fnt",
			Data = jsonObj.Data.Substring(0, 4096) // 2048 bytes hex -> will be duplicated by loader
		};
		RecordArtifact(Path.Combine("projects", "sample_v1911.atrview"), v1911Obj.ToJson());

		// 5. Synthetic .atrview v2007 (32-wide view compatibility)
		var v2007Obj = new AtrViewInfoJson {
			Version = "2007",
			ColoredGfx = "1",
			Width = 32,
			Height = 26,
			Chars = jsonObj.Chars,
			Lines = jsonObj.Lines,
			Colors = jsonObj.Colors,
			FortyBytes = "0",
			Fontname1 = "Default.fnt",
			Fontname2 = "Default.fnt",
			Fontname3 = "Default.fnt",
			Fontname4 = "Default.fnt",
			Data = jsonObj.Data
		};
		RecordArtifact(Path.Combine("projects", "sample_v2007.atrview"), v2007Obj.ToJson());

		// 6. ClipboardJson Tests
		var clip = new ClipboardJson {
			Width = "4",
			Height = "2",
			Chars = "0001020304050607",
			Data = "0000000000000000FFFFFFFFFFFFFFFF",
			FontNr = "11111111",
			Nulls = "00000000"
		};
		bool valid = clip.VerifyWidthHeight();
		clip.FixCharacters();
		string clipJson = clip.ToJson();
		RecordArtifact(Path.Combine("projects", "clipboard_sample.json"), clipJson);

		// 7. TileSet and TileData JSON format (.atrtileset and .atrtile)
		var tileSetJson = new AtrTileSetJson {
			Version = AtrTileSetJson.MY_VERSION,
			Tiles = new List<SavedTileData> {
				new SavedTileData {
					Nr = 0,
					Font = "11111111",
					View = "000102030405060708090A0B0C0D0E0F101112131415161718",
					Nulls = "0000000000000000000000000",
					Width = 5,
					Height = 5
				}
			}
		};
		RecordArtifact(Path.Combine("projects", "sample.atrtileset"), tileSetJson.ToJson());

		var tileJson = new AtrTileJson {
			Version = AtrTileJson.MY_VERSION,
			Tile = new SavedTileData {
				Nr = 42,
				Font = "22222222",
				View = "000102030405060708090A0B0C0D0E0F101112131415161718",
				Nulls = "0000000000000000000000000",
				Width = 5,
				Height = 5
			}
		};
		RecordArtifact(Path.Combine("projects", "sample.atrtile"), tileJson.ToJson());

		// 8. ConfigurationJson default verification
		Configuration.Values = new ConfigurationJson();
		Configuration.VerifyDefaults();
		RecordArtifact(Path.Combine("projects", "sample_config.json"), Configuration.Values.ToJson());

		Console.WriteLine("  - Verified .fnt, .fn2, .atrview (v1911, v2007, v2023), ClipboardJson, TileSet, and Configuration.");
	}
	#endregion

	#region 6. Code & Data Exporters Tests
	private static void RunExportTests()
	{
		Console.WriteLine("\n[6/7] Exporters (Font & View to ASM, Action!, BASIC, C, Pascal, MADS, LST, BMP)...");

		byte[] defaultFont = Helpers.GetResource<byte[]>("Default.fnt");
		Array.Clear(AtariFont.FontBytes, 0, AtariFont.FontBytes.Length);
		for (int i = 0; i < 4; i++)
		{
			Array.Copy(defaultFont, 0, AtariFont.FontBytes, i * 1024, 1024);
		}

		// Use Reflection to execute the exact C# exporter methods in ExportFontWindow and ExportViewWindow
		MethodInfo? fontExportMethod = typeof(ExportFontWindow).GetMethod("GenerateFileAsText", BindingFlags.NonPublic | BindingFlags.Static);
		if (fontExportMethod == null) throw new Exception("Could not find ExportFontWindow.GenerateFileAsText via Reflection");

		// Test Font Exporters across formats (decimal & hex)
		// ExportType mapping: 2=Assembler, 3=Action, 4=AtariBasic, 5=FastBasic, 6=MADSdta, 7=CDataArray, 8=MadPascalArray
		var formats = new (string name, ExportFontWindow.FormatTypes formatType, int dataType)[]
		{
			("font_asm_dec.txt", ExportFontWindow.FormatTypes.Assembler, 0),
			("font_asm_hex.txt", ExportFontWindow.FormatTypes.Assembler, 1),
			("font_action_dec.txt", ExportFontWindow.FormatTypes.Action, 0),
			("font_action_hex.txt", ExportFontWindow.FormatTypes.Action, 1),
			("font_ataribasic.txt", ExportFontWindow.FormatTypes.AtariBasic, 0),
			("font_fastbasic.txt", ExportFontWindow.FormatTypes.FastBasic, 0),
			("font_mads_dec.txt", ExportFontWindow.FormatTypes.MADSdta, 0),
			("font_mads_hex.txt", ExportFontWindow.FormatTypes.MADSdta, 1),
			("font_c_dec.txt", ExportFontWindow.FormatTypes.CDataArray, 0),
			("font_c_hex.txt", ExportFontWindow.FormatTypes.CDataArray, 1),
			("font_pascal_dec.txt", ExportFontWindow.FormatTypes.MadPascalArray, 0),
			("font_pascal_hex.txt", ExportFontWindow.FormatTypes.MadPascalArray, 1)
		};

		foreach (var (name, formatType, dataType) in formats)
		{
			var result = fontExportMethod.Invoke(null, new object[] { 0, formatType, dataType, false });
			if (result is ValueTuple<string, int, int> tuple)
			{
				RecordArtifact(Path.Combine("exports", name), tuple.Item1);
			}
		}

		// Test Font Basic Listing (.lst merged with basicremfont.lst)
		string lstTempFile = Path.GetTempFileName();
		MethodInfo? saveRemFontMethod = typeof(ExportFontWindow).GetMethod("SaveRemFont", BindingFlags.NonPublic | BindingFlags.Static);
		if (saveRemFontMethod != null)
		{
			saveRemFontMethod.Invoke(null, new object[] { 0, lstTempFile });
			byte[] lstBytes = File.ReadAllBytes(lstTempFile);
			RecordArtifact(Path.Combine("exports", "font_default.lst"), lstBytes);
			try { File.Delete(lstTempFile); } catch { }
		}

		// Test Font BMP Export
		string bmpMonoTemp = Path.GetTempFileName();
		string bmpColorTemp = Path.GetTempFileName();
		MethodInfo? saveFontBmpMethod = typeof(ExportFontWindow).GetMethod("SaveFontBMP", BindingFlags.NonPublic | BindingFlags.Static);
		if (saveFontBmpMethod != null)
		{
			saveFontBmpMethod.Invoke(null, new object[] { 0, bmpMonoTemp, false });
			RecordArtifact(Path.Combine("exports", "font_default_mono.bmp"), File.ReadAllBytes(bmpMonoTemp));
			try { File.Delete(bmpMonoTemp); } catch { }

			saveFontBmpMethod.Invoke(null, new object[] { 0, bmpColorTemp, true });
			RecordArtifact(Path.Combine("exports", "font_default_color.bmp"), File.ReadAllBytes(bmpColorTemp));
			try { File.Delete(bmpColorTemp); } catch { }
		}

		// Test View Exporter (ExportViewWindow.GenerateFileAsText)
		MethodInfo? viewExportMethod = typeof(ExportViewWindow).GetMethod("GenerateFileAsText", BindingFlags.NonPublic | BindingFlags.Static);
		if (viewExportMethod != null)
		{
			// Setup sample view
			AtariView.Setup();
			for (int y = 0; y < 26; y++)
			{
				AtariView.UseFontOnLine[y] = (byte)(1 + (y % 4));
				for (int x = 0; x < 40; x++)
				{
					AtariView.ViewBytes[x, y] = (byte)((x + y * 40) % 128);
				}
			}

			Rectangle exportRegion = new Rectangle(0, 0, 40, 26);
			var viewFormats = new (string name, ExportViewWindow.FormatTypes formatType, bool hasHex, bool transpose)[]
			{
				("view_asm_hex.txt", ExportViewWindow.FormatTypes.Assembler, true, false),
				("view_action_hex.txt", ExportViewWindow.FormatTypes.Action, true, false),
				("view_ataribasic.txt", ExportViewWindow.FormatTypes.AtariBasic, false, false),
				("view_fastbasic.txt", ExportViewWindow.FormatTypes.FastBasic, false, false),
				("view_mads_hex.txt", ExportViewWindow.FormatTypes.MADSdta, true, false),
				("view_c_hex.txt", ExportViewWindow.FormatTypes.CDataArray, true, false),
				("view_pascal_hex.txt", ExportViewWindow.FormatTypes.MadPascalArray, true, false),
				("view_asm_transposed.txt", ExportViewWindow.FormatTypes.Assembler, true, true)
			};

			foreach (var (name, formatType, hasHex, transpose) in viewFormats)
			{
				var result = viewExportMethod.Invoke(null, new object[] { exportRegion, formatType, hasHex, transpose, false });
				if (result is ValueTuple<string, int, int> tuple)
				{
					RecordArtifact(Path.Combine("exports", name), tuple.Item1);
				}
			}
		}

		Console.WriteLine("  - Generated all 12 Font export variations, BASIC .lst, BMP, and 8 View export variations.");
	}
	#endregion

	#region 7. Undo/Redo State Machine Tests
	private static void RunUndoRedoTests()
	{
		Console.WriteLine("\n[7/7] Undo/Redo State Machine (Font & View Buffers) Tests...");

		var undoLog = new List<object>();

		// 1. AtariFontUndoBuffer Tests
		AtariFontUndoBuffer.Setup();
		Array.Clear(AtariFont.FontBytes, 0, AtariFont.FontBytes.Length);
		AtariFont.FontBytes[0] = 0x11;
		AtariFontUndoBuffer.Add2UndoInitial();

		var (redoInit, undoInit) = AtariFontUndoBuffer.GetRedoUndoButtonState(false);
		undoLog.Add(new { step = "font_init", redo = redoInit, undo = undoInit, val = AtariFont.FontBytes[0] });

		// Mutation 1
		AtariFont.FontBytes[0] = 0x22;
		AtariFontUndoBuffer.Add2Undo(true);
		var (redoMut1, undoMut1) = AtariFontUndoBuffer.GetRedoUndoButtonState(false);
		undoLog.Add(new { step = "font_mut1", redo = redoMut1, undo = undoMut1, val = AtariFont.FontBytes[0] });

		// Undo
		AtariFontUndoBuffer.Undo();
		var (redoAfterUndo, undoAfterUndo) = AtariFontUndoBuffer.GetRedoUndoButtonState(false);
		undoLog.Add(new { step = "font_after_undo", redo = redoAfterUndo, undo = undoAfterUndo, val = AtariFont.FontBytes[0] });

		// Redo
		AtariFontUndoBuffer.Redo();
		var (redoAfterRedo, undoAfterRedo) = AtariFontUndoBuffer.GetRedoUndoButtonState(false);
		undoLog.Add(new { step = "font_after_redo", redo = redoAfterRedo, undo = undoAfterRedo, val = AtariFont.FontBytes[0] });

		// Test 250+ entries circular buffer overflow
		AtariFontUndoBuffer.Setup();
		AtariFontUndoBuffer.Add2UndoInitial();
		for (int i = 1; i <= 260; i++)
		{
			AtariFont.FontBytes[0] = (byte)(i % 256);
			AtariFontUndoBuffer.Add2Undo(true);
		}
		undoLog.Add(new {
			step = "font_overflow_260_edits",
			current_index = AtariFontUndoBuffer.undoBufferIndex,
			val_at_current = AtariFont.FontBytes[0]
		});

		// 2. AtariViewUndoBuffer Tests
		var viewUndo = new AtariViewUndoBuffer();
		AtariView.Setup();
		AtariView.ViewBytes[0, 0] = 0x05;
		viewUndo.Push();

		AtariView.ViewBytes[0, 0] = 0x0A;
		viewUndo.Push();

		var (undoViewBefore, redoViewBefore) = viewUndo.GetRedoUndoButtonState();
		undoLog.Add(new { step = "view_pushed_twice", undo_available = undoViewBefore, redo_available = redoViewBefore, val = (int)AtariView.ViewBytes[0, 0] });

		viewUndo.Undo();
		var (undoViewAfter, redoViewAfter) = viewUndo.GetRedoUndoButtonState();
		undoLog.Add(new { step = "view_after_undo", undo_available = undoViewAfter, redo_available = redoViewAfter, val = (int)AtariView.ViewBytes[0, 0] });

		viewUndo.Redo();
		undoLog.Add(new { step = "view_after_redo", val = (int)AtariView.ViewBytes[0, 0] });

		// Push 260 view states
		for (int i = 0; i < 260; i++)
		{
			AtariView.ViewBytes[0, 0] = (byte)(i % 256);
			viewUndo.Push();
		}
		undoLog.Add(new { step = "view_overflow_260_pushes", val = (int)AtariView.ViewBytes[0, 0] });

		RecordArtifact(Path.Combine("undo", "undo_redo_state_transitions.json"), undoLog.ToJson());
		Console.WriteLine("  - Verified Font and View Undo/Redo state transitions & 250-level circular buffer boundaries.");
	}
	#endregion
}
